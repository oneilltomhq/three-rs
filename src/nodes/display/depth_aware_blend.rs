//! Port of `three.js/examples/jsm/tsl/display/depthAwareBlend.js` — blends a
//! low-resolution effect over a full-resolution image without haloing at
//! depth edges.
//!
//! Eight Poisson-disk taps around each pixel find the neighbours whose linear
//! depth is within 5% of the pixel's own; the effect is then read at the
//! pixel's uv pushed toward their average offset, so a ray computed at half
//! resolution is sampled from the same surface, not from across a silhouette.
//! The effect's red channel is the blend factor toward `blendColor`.
//!
//! Faithful quirks: `pushDir.divAssign( count ).normalize()` discards the
//! `normalize()`, so the push is the average offset, not a unit vector; and
//! `edgeRadius`, an `int` uniform on the page, is read as an `f32`, which is
//! how three's WGSL declares it.
//!
//! Not ported: an orthographic camera (three assumes a perspective one too —
//! `perspectiveDepthToViewZ` is hard-wired), and a `baseNode` with its own
//! `uvNode`: the port's base is a texture read at `uv()`.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::cameras::PerspectiveCamera;
use crate::math::Color;
use crate::nodes::node::{LiveValue, TextureSource, Type, UniformGroup, UniformSource};
use crate::nodes::tsl::{
    abs, block, const_array_of, float, if_then, int, length, loop_n, mix, pass_depth_texture_uv,
    perspective_depth_to_view_z, texture_size, texture_uv, to_const, to_var, uniform, uv, vec2,
    vec3, vec4_join, view_z_to_orthographic_depth,
};
use crate::nodes::NodeRef;
use crate::textures::{DepthTexture, Texture};

/// The eight-tap Poisson disk of `depthAwareBlend`.
const POISSON_DISK: [[f64; 2]; 8] = [
    [0.493393, 0.394269],
    [0.798547, 0.885922],
    [0.259143, 0.650754],
    [0.605322, 0.023588],
    [-0.574681, 0.137452],
    [-0.430397, -0.638423],
    [-0.849487, -0.366258],
    [0.170621, -0.569941],
];

/// `depthAwareBlend( …, options )`' `options` object.
#[derive(Clone)]
#[non_exhaustive]
pub struct DepthAwareBlendOptions {
    /// `options.blendColor` — `color( 0xffffff )` by default.
    pub blend_color: NodeRef,
    /// `options.edgeRadius` — `int( 2 )`: the Poisson disk's radius, in
    /// texels of the base.
    pub edge_radius: NodeRef,
    /// `options.edgeStrength` — `float( 2 )`: how far, in texels, the effect
    /// is read along the push direction.
    pub edge_strength: NodeRef,
}

impl Default for DepthAwareBlendOptions {
    fn default() -> Self {
        let white = Color::from_hex(0xffffff);
        Self {
            blend_color: vec3(white.r, white.g, white.b),
            edge_radius: int(2),
            edge_strength: float(2.0),
        }
    }
}

/// `reference( name, 'float', camera )` — the camera's value at draw time.
fn camera_reference(
    camera: &Rc<RefCell<PerspectiveCamera>>,
    read: fn(&PerspectiveCamera) -> f64,
) -> NodeRef {
    let camera: Weak<RefCell<PerspectiveCamera>> = Rc::downgrade(camera);
    uniform(
        UniformSource::Live(LiveValue::new(move || {
            let camera = camera
                .upgrade()
                .expect("three-rs: a depthAwareBlend outlived its camera");
            let value = read(&camera.borrow());
            vec![value]
        })),
        Type::F32,
        UniformGroup::Object,
        None,
    )
}

/// `depthAwareBlend( baseNode, blendNode, depthNode, camera, options )`.
///
/// `base` is the full-resolution image (a pass's `output`), `blend` the
/// effect whose red channel is the blend factor, `depth` the pass's depth
/// attachment and `camera` the pass's camera.
pub fn depth_aware_blend(
    base: &Texture,
    blend: &Texture,
    depth: &DepthTexture,
    camera: &Rc<RefCell<PerspectiveCamera>>,
    options: DepthAwareBlendOptions,
) -> NodeRef {
    let uv_node = uv();
    let camera_near = camera_reference(camera, |camera| camera.near);
    let camera_far = camera_reference(camera, |camera| camera.far);

    let view_z = perspective_depth_to_view_z(
        pass_depth_texture_uv(depth, uv_node.clone()),
        camera_near.clone(),
        camera_far.clone(),
    );
    let correct_depth =
        view_z_to_orthographic_depth(view_z, camera_near.clone(), camera_far.clone());

    let push_dir = to_var(None, vec2(0.0, 0.0));
    let count = to_var(None, float(0.0));
    // `ivec2( textureSize( baseNode ) ).toConst()`.
    let resolution = to_const(
        None,
        texture_size(TextureSource::Texture2D(base.clone()), int(0)).to(Type::IVec2),
    );
    let pixel_step = vec2(1.0, 1.0).div(resolution.clone());
    let poisson_disk = const_array_of(Type::Vec2, POISSON_DISK.iter().flatten().copied().collect());

    let taps = loop_n("i", int(8), |i| {
        let offset = poisson_disk
            .element_node(i.clone())
            .mul(options.edge_radius.clone());
        let sample_uv = uv_node.clone().add(offset.clone().mul(pixel_step));
        let sample_depth = pass_depth_texture_uv(depth, sample_uv);
        let sample_view_z =
            perspective_depth_to_view_z(sample_depth, camera_near.clone(), camera_far.clone());
        let sample_linear_depth =
            view_z_to_orthographic_depth(sample_view_z, camera_near.clone(), camera_far.clone());
        vec![if_then(
            abs(sample_linear_depth.sub(correct_depth.clone()))
                .less_than(float(0.05).mul(correct_depth.clone())),
            vec![push_dir.add_assign(offset), count.add_assign(float(1.0))],
        )]
    });

    let count_fix = count.assign(count.equal(float(0.0)).select(float(1.0), count.clone()));
    // `pushDir.divAssign( count ).normalize()` — the `normalize()` result is
    // dropped; see the module docs.
    let push = push_dir.div_assign(count.clone());

    let sample_uv = length(push_dir.clone()).greater_than(float(0.0)).select(
        uv_node.clone().add(
            options
                .edge_strength
                .mul(push_dir.clone().div(resolution.clone())),
        ),
        uv_node.clone(),
    );
    let best_choice = texture_uv(blend, sample_uv).x();
    let base_color = texture_uv(base, uv_node);

    block(
        vec![
            push_dir.clone(),
            count.clone(),
            resolution,
            taps,
            count_fix,
            push,
        ],
        mix(
            base_color,
            vec4_join(vec![options.blend_color, float(1.0)]),
            best_choice,
        ),
    )
}
