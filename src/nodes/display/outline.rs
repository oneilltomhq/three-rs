//! Port of `three.js/examples/jsm/tsl/display/OutlineNode.js` — the selection
//! outline of `webgpu_postprocessing_outline`.
//!
//! Every frame with a non-empty selection, `updateBefore()` renders the scene
//! twice through a render-object function and then draws seven quads:
//!
//! 1. every object *not* selected, with a black depth-only material, into a
//!    target whose `DepthTexture` (`FloatType`) the next step reads;
//! 2. only the selected objects, with `prepareMask`, which writes
//!    `vec3( 0, depthTest, 1 )` — `depthTest` is 1 where the fragment is
//!    behind the non-selected depth (hidden) and 0 where it is in front of it
//!    (the white clear leaves 1 everywhere outside the selection);
//! 3. a copy of that mask into a target at `1 / downSampleRatio` of the
//!    drawing buffer (`Math.round`ed);
//! 4. edge detection: four taps one texel apart, `d` the length of the two
//!    central differences of the mask's red channel, red where any tap is
//!    visible (its green below 1) and green where all are hidden —
//!    `visibleEdge` is the `.r` of the result and `hiddenEdge` the `.g`;
//! 5. a separable Gaussian of radius `edgeThickness` (`MAX_RADIUS = 4` taps a
//!    side) at that resolution, X into the blur target and Y back into the
//!    edge target;
//! 6. the same blur at radius 4 at half that resolution again;
//! 7. `mask.r * ( edge1 + edge2 * edgeGlow )` into the full-size composite,
//!    which [`OutlineNode::node`] reads (`passTexture( this,
//!    composite.texture )`).
//!
//! With an empty selection nothing is drawn; on the frame the selection
//! *becomes* empty the composite is cleared to transparent black once, so a
//! stale outline does not stay on screen.
//!
//! The scene renders reach the renderer through
//! `Renderer.outline_selection`, the port's spelling of the two
//! `setRenderObjectFunction()` callbacks (see `docs/nodes.md` §72).
//!
//! Differences from three.js, none of which changes a pixel:
//!
//! - three has *one* blur material per resolution whose colour texture and
//!   `_blurDirection` uniform it rewrites between the X and Y draws; the port
//!   has one material per draw, each bound to its own source texture with its
//!   direction baked in, as [`bloom`](super::bloom) already does for
//!   `UnrealBloomPass`. The four programs are the same two `Fn()` bodies;
//! - `_cameraNear` / `_cameraFar` are `reference( 'near', 'float', camera )`
//!   in three, refreshed per object; here they are settable uniforms written
//!   from the camera at the top of each `updateBefore()`;
//! - `scene.name` is not renamed for the two passes (it only labels them in
//!   three's inspector), and `_quadMesh.name` likewise.
//!
//! Not ported: `dispose()`; reassigning `downSampleRatio`, `edgeThicknessNode`
//! or `edgeGlowNode` after construction (three would pick a new node up at
//! the next `setup()`; the port builds the materials once); and `getTextureNode()`
//! as a separate texture object — [`OutlineNode::node`] *is* the composite tap.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use crate::core::Node;
use crate::materials::MeshBasicNodeMaterial;
use crate::math::Color;
use crate::nodes::node::{SettableValue, TextureSource, Type};
use crate::nodes::tsl::{
    block, depth_texture_sample, exp, float, int, length, loop_options, min_of,
    perspective_depth_to_view_z, position_view, screen_uv, texture, texture_size, texture_uv,
    texture_with_uv, to_var, uniform_settable, uniform_value, uv, vec2, vec2_join, vec3, vec3_join,
    vec4, vec4_join,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{
    CameraRef, OutlineSelection, RenderTarget, RenderTargetOptions, Renderer, SceneRef,
};
use crate::textures::{DepthTexture, Texture, TextureType};

/// `MAX_RADIUS` in `OutlineNode.setup()`: the blur's taps per side.
const MAX_RADIUS: f64 = 4.0;

/// `outline( scene, camera, params )`'s `params`, with its defaults.
pub struct OutlineParams {
    /// `params.selectedObjects`, default `[]`.
    pub selected_objects: Vec<Node>,
    /// `params.edgeThickness`, default `float( 1 )` — the half-resolution
    /// blur's kernel radius.
    pub edge_thickness: NodeRef,
    /// `params.edgeGlow`, default `float( 0 )` — how much of the
    /// quarter-resolution blur is added to the composite.
    pub edge_glow: NodeRef,
    /// `params.downSampleRatio`, default `2`.
    pub down_sample_ratio: f64,
}

impl Default for OutlineParams {
    fn default() -> Self {
        Self {
            selected_objects: Vec::new(),
            edge_thickness: float(1.0),
            edge_glow: float(0.0),
            down_sample_ratio: 2.0,
        }
    }
}

/// `outline( scene, camera, params )`.
pub fn outline(scene: SceneRef, camera: CameraRef, params: OutlineParams) -> OutlineNode {
    OutlineNode::new(scene, camera, params)
}

/// `OutlineNode`: a handle; clones share the render targets and the selection.
#[derive(Clone)]
pub struct OutlineNode(Rc<OutlineState>);

impl std::ops::Deref for OutlineNode {
    type Target = OutlineState;

    fn deref(&self) -> &OutlineState {
        &self.0
    }
}

/// What an [`OutlineNode`] handle shares.
pub struct OutlineState {
    scene: SceneRef,
    camera: CameraRef,
    /// `this.selectedObjects`.
    selected_objects: RefCell<Vec<Node>>,
    /// `this.downSampleRatio`.
    down_sample_ratio: f64,
    depth_buffer: RenderTarget,
    mask_buffer: RenderTarget,
    mask_down_sample_buffer: RenderTarget,
    edge_buffer1: RenderTarget,
    edge_buffer2: RenderTarget,
    blur_buffer1: RenderTarget,
    blur_buffer2: RenderTarget,
    composite_buffer: RenderTarget,
    /// `this._cameraNear` / `this._cameraFar`.
    camera_near: SettableValue,
    camera_far: SettableValue,
    depth_material: Rc<MeshBasicNodeMaterial>,
    depth_sprite_material: Rc<MeshBasicNodeMaterial>,
    prepare_mask_material: Rc<MeshBasicNodeMaterial>,
    prepare_mask_sprite_material: Rc<MeshBasicNodeMaterial>,
    copy: QuadMesh,
    edge_detection: QuadMesh,
    /// `_separableBlurMaterial` drawn X then Y, and `_separableBlurMaterial2`
    /// drawn X then Y — one material per draw, see the module docs.
    blur_half: [QuadMesh; 2],
    blur_quarter: [QuadMesh; 2],
    composite: QuadMesh,
    /// `this._lastSelectionCount`.
    last_selection_count: Cell<usize>,
    /// `this._textureNode`, the composite tap.
    node: NodeRef,
}

/// `ViewportDepthNode.js`' `orthographicDepthToViewZ( depth, near, far )`:
/// `( near - far ) * depth - near`.
fn orthographic_depth_to_view_z(depth: NodeRef, near: NodeRef, far: NodeRef) -> NodeRef {
    near.clone().sub(far).mul(depth).sub(near)
}

/// `gaussianPdf( x, sigma )`: `0.39894 * exp( -0.5 * x * x / ( sigma * sigma
/// ) ) / sigma`. A layout-less `Fn()`, so it is inlined at each call.
fn gaussian_pdf(x: NodeRef, sigma: NodeRef) -> NodeRef {
    float(0.39894).mul(
        exp(float(-0.5)
            .mul(x.clone())
            .mul(x)
            .div(sigma.clone().mul(sigma.clone())))
        .div(sigma),
    )
}

/// `vec2( 1 ).div( textureSize( maskDownSample ) ).toVar()`.
fn inv_size(mask_down_sample: &Texture) -> NodeRef {
    to_var(
        None,
        vec2(1.0, 1.0).div(
            texture_size(TextureSource::Texture2D(mask_down_sample.clone()), int(0)).to(Type::Vec2),
        ),
    )
}

/// `edgeDetection()`.
fn edge_detection(mask_down_sample: &Texture) -> NodeRef {
    let inv_size = inv_size(mask_down_sample);
    let uv_offset =
        vec4(1.0, 0.0, 0.0, 1.0).mul(vec4_join(vec![inv_size.clone(), inv_size.clone()]));
    let uv_node = uv();
    let tap = |coord: NodeRef| to_var(None, texture_with_uv(mask_down_sample, coord));
    let c1 = tap(uv_node.clone().add(uv_offset.xy()));
    let c2 = tap(uv_node.clone().sub(uv_offset.xy()));
    let c3 = tap(uv_node.clone().add(uv_offset.swizzle("yw")));
    let c4 = tap(uv_node.sub(uv_offset.swizzle("yw")));
    let diff1 = c1.x().sub(c2.x()).mul(float(0.5));
    let diff2 = c3.x().sub(c4.x()).mul(float(0.5));
    let d = length(vec2_join(vec![diff1, diff2]));
    let a1 = min_of(c1.y(), c2.y());
    let a2 = min_of(c3.y(), c4.y());
    let visibility_factor = min_of(a1, a2);
    // `this._visibleEdgeColor` / `this._hiddenEdgeColor`.
    let edge_color = visibility_factor
        .one_minus()
        .greater_than(float(0.001))
        .select(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0));
    block(
        vec![inv_size, c1, c2, c3, c4],
        vec4_join(vec![edge_color, float(1.0)]).mul(d),
    )
}

/// `separableBlur( kernelRadius )`, reading `color` along `direction`.
fn separable_blur(
    mask_down_sample: &Texture,
    color: &Texture,
    direction: [f64; 2],
    kernel_radius: NodeRef,
) -> NodeRef {
    let inv_size = inv_size(mask_down_sample);
    let uv_node = uv();
    let sigma = to_var(None, kernel_radius.div(float(2.0)));
    let weight_sum = to_var(None, gaussian_pdf(float(0.0), sigma.clone()));
    let diffuse_sum = to_var(
        None,
        texture_with_uv(color, uv_node.clone()).mul(weight_sum.clone()),
    );
    // `this._blurDirection`, whose value three copies in before each draw.
    let blur_direction = uniform_value(Type::Vec2, direction.to_vec());
    let delta = to_var(
        None,
        blur_direction
            .mul(inv_size.clone())
            .mul(kernel_radius.clone())
            .div(float(MAX_RADIUS)),
    );
    let uv_offset = to_var(None, delta.clone());

    let sigma_var = sigma.clone();
    let body = {
        let (diffuse_sum, weight_sum, uv_offset, delta) = (
            diffuse_sum.clone(),
            weight_sum.clone(),
            uv_offset.clone(),
            delta.clone(),
        );
        move |i: &NodeRef| {
            let x = kernel_radius.mul(i.to(Type::F32)).div(float(MAX_RADIUS));
            let w = gaussian_pdf(x, sigma.clone());
            let sample1 = texture_with_uv(color, uv_node.clone().add(uv_offset.clone()));
            let sample2 = texture_with_uv(color, uv_node.sub(uv_offset.clone()));
            vec![
                diffuse_sum.add_assign(sample1.add(sample2).mul(w.clone())),
                weight_sum.add_assign(w.mul(float(2.0))),
                uv_offset.add_assign(delta),
            ]
        }
    };
    let blur_loop = loop_options("i", Type::I32, int(1), int(MAX_RADIUS as i64), "<=", body);

    block(
        vec![
            inv_size,
            sigma_var,
            weight_sum.clone(),
            diffuse_sum.clone(),
            delta,
            uv_offset,
            blur_loop,
        ],
        diffuse_sum.div(weight_sum),
    )
}

/// A colour target with no depth buffer: `new RenderTarget( 1, 1, {
/// depthBuffer: false } )`.
fn colour_target(depth_buffer: bool) -> RenderTarget {
    RenderTarget::new_with_options(
        1,
        1,
        RenderTargetOptions {
            depth_buffer,
            ..RenderTargetOptions::default()
        },
    )
    .expect("three-rs: an outline render target is an UnsignedByte colour type")
}

fn quad(name: &'static str, fragment: NodeRef) -> QuadMesh {
    let mut material = MeshBasicNodeMaterial::new();
    material.name = name;
    material.fragment_node = Some(fragment);
    QuadMesh::new(material)
}

impl OutlineNode {
    /// `new OutlineNode( scene, camera, params )` and its `setup()`.
    pub fn new(scene: SceneRef, camera: CameraRef, params: OutlineParams) -> Self {
        let OutlineParams {
            selected_objects,
            edge_thickness,
            edge_glow,
            down_sample_ratio,
        } = params;

        // render targets
        let depth_buffer = RenderTarget::new(1, 1);
        let depth_texture = DepthTexture::new();
        depth_texture
            .set_type(TextureType::Float)
            .expect("three-rs: FloatType is a depth type");
        depth_buffer.set_depth_texture(depth_texture.clone());
        let mask_buffer = RenderTarget::new(1, 1);
        let mask_down_sample_buffer = colour_target(false);
        let edge_buffer1 = colour_target(false);
        let edge_buffer2 = colour_target(false);
        let blur_buffer1 = colour_target(false);
        let blur_buffer2 = colour_target(false);
        let composite_buffer = colour_target(false);

        // uniforms
        let (is_perspective, near, far) = {
            let camera = camera.borrow();
            (camera.is_perspective_camera(), camera.near(), camera.far())
        };
        let (camera_near_node, camera_near) = uniform_settable(Type::F32, vec![near]);
        let (camera_far_node, camera_far) = uniform_settable(Type::F32, vec![far]);

        let mask = mask_buffer.texture();
        let mask_down_sample = mask_down_sample_buffer.texture();

        // materials
        let mut depth_material = MeshBasicNodeMaterial::new();
        depth_material.color_node = Some(vec3(0.0, 0.0, 0.0));
        depth_material.name = "OutlineNode.depth";
        let mut depth_sprite_material = MeshBasicNodeMaterial::sprite();
        depth_sprite_material.color_node = Some(vec3(0.0, 0.0, 0.0));
        depth_sprite_material.name = "OutlineNode.depthSprite";

        // prepare mask material
        let prepare_mask = || {
            let depth = depth_texture_sample(&depth_texture, screen_uv());
            let view_z = if is_perspective {
                perspective_depth_to_view_z(
                    depth,
                    camera_near_node.clone(),
                    camera_far_node.clone(),
                )
            } else {
                orthographic_depth_to_view_z(
                    depth,
                    camera_near_node.clone(),
                    camera_far_node.clone(),
                )
            };
            let depth_test = position_view()
                .z()
                .less_than_equal(view_z)
                .select(float(1.0), float(0.0));
            vec3_join(vec![float(0.0), depth_test, float(1.0)])
        };
        let mut prepare_mask_material = MeshBasicNodeMaterial::new();
        prepare_mask_material.name = "OutlineNode.prepareMask";
        prepare_mask_material.color_node = Some(prepare_mask());
        let mut prepare_mask_sprite_material = MeshBasicNodeMaterial::sprite();
        prepare_mask_sprite_material.name = "OutlineNode.prepareMaskSprite";
        prepare_mask_sprite_material.color_node = Some(prepare_mask());

        // copy material
        let copy = quad("OutlineNode.copy", texture(&mask));

        // edge detection material
        let edge_detection = quad(
            "OutlineNode.edgeDetection",
            edge_detection(&mask_down_sample),
        );

        // separable blur materials
        let blur = |name, color: &Texture, direction, radius: &NodeRef| {
            quad(
                name,
                separable_blur(&mask_down_sample, color, direction, radius.clone()),
            )
        };
        let quarter_radius = float(MAX_RADIUS);
        let blur_half = [
            blur(
                "OutlineNode.separableBlur",
                &edge_buffer1.texture(),
                [1.0, 0.0],
                &edge_thickness,
            ),
            blur(
                "OutlineNode.separableBlur",
                &blur_buffer1.texture(),
                [0.0, 1.0],
                &edge_thickness,
            ),
        ];
        let blur_quarter = [
            blur(
                "OutlineNode.separableBlur2",
                &edge_buffer1.texture(),
                [1.0, 0.0],
                &quarter_radius,
            ),
            blur(
                "OutlineNode.separableBlur2",
                &blur_buffer2.texture(),
                [0.0, 1.0],
                &quarter_radius,
            ),
        ];

        // composite material
        let edge_value =
            texture(&edge_buffer1.texture()).add(texture(&edge_buffer2.texture()).mul(edge_glow));
        let composite = quad("OutlineNode.composite", texture(&mask).x().mul(edge_value));

        // `passTexture( this, this._renderTargetComposite.texture )`: a
        // `PassTextureNode` turns its uv matrix off.
        let node = texture_uv(&composite_buffer.texture(), uv());

        let outline = Self(Rc::new(OutlineState {
            scene,
            camera,
            selected_objects: RefCell::new(selected_objects),
            down_sample_ratio,
            depth_buffer,
            mask_buffer,
            mask_down_sample_buffer,
            edge_buffer1,
            edge_buffer2,
            blur_buffer1,
            blur_buffer2,
            composite_buffer,
            camera_near,
            camera_far,
            depth_material: Rc::new(depth_material),
            depth_sprite_material: Rc::new(depth_sprite_material),
            prepare_mask_material: Rc::new(prepare_mask_material),
            prepare_mask_sprite_material: Rc::new(prepare_mask_sprite_material),
            copy,
            edge_detection,
            blur_half,
            blur_quarter,
            composite,
            last_selection_count: Cell::new(0),
            node,
        }));
        crate::nodes::frame::register_texture_update(
            outline.composite_buffer.texture().id(),
            &outline.0,
        );
        outline
    }

    /// The node itself — `vec4`, the composite: red where the outline is
    /// visible, green where it is hidden.
    pub fn node(&self) -> NodeRef {
        self.node.clone()
    }

    /// `outlinePass.visibleEdge` — `this.r`.
    pub fn visible_edge(&self) -> NodeRef {
        self.node.x()
    }

    /// `outlinePass.hiddenEdge` — `this.g`.
    pub fn hidden_edge(&self) -> NodeRef {
        self.node.y()
    }

    /// The materials three's dump lists for this node, in its order: depth,
    /// depth sprite, prepare mask, prepare mask sprite, copy, edge detection,
    /// the two half-resolution blurs, the two quarter-resolution blurs, and
    /// the composite. For `tests/nodes_display_wgsl.rs`.
    #[doc(hidden)]
    pub fn materials(&self) -> Vec<&MeshBasicNodeMaterial> {
        vec![
            &self.depth_material,
            &self.depth_sprite_material,
            &self.prepare_mask_material,
            &self.prepare_mask_sprite_material,
            &self.copy.material,
            &self.edge_detection.material,
            &self.blur_half[0].material,
            &self.blur_half[1].material,
            &self.blur_quarter[0].material,
            &self.blur_quarter[1].material,
            &self.composite.material,
        ]
    }
}

impl OutlineState {
    /// `outlinePass.selectedObjects` — a copy of the list.
    pub fn selected_objects(&self) -> Vec<Node> {
        self.selected_objects.borrow().clone()
    }

    /// `outlinePass.selectedObjects = objects` (or the page's `length = 0`
    /// followed by `push()`).
    pub fn set_selected_objects(&self, objects: Vec<Node>) {
        *self.selected_objects.borrow_mut() = objects;
    }

    /// `OutlineNode.downSampleRatio`.
    pub fn down_sample_ratio(&self) -> f64 {
        self.down_sample_ratio
    }

    /// `OutlineNode.setSize( width, height )`.
    pub fn set_size(&self, width: u32, height: u32) {
        self.depth_buffer.set_size(width, height);
        self.mask_buffer.set_size(width, height);
        self.composite_buffer.set_size(width, height);

        // downsample 1
        let resx = (f64::from(width) / self.down_sample_ratio).round();
        let resy = (f64::from(height) / self.down_sample_ratio).round();
        self.mask_down_sample_buffer
            .set_size(resx as u32, resy as u32);
        self.edge_buffer1.set_size(resx as u32, resy as u32);
        self.blur_buffer1.set_size(resx as u32, resy as u32);

        // downsample 2
        let resx = (resx / 2.0).round() as u32;
        let resy = (resy / 2.0).round() as u32;
        self.edge_buffer2.set_size(resx, resy);
        self.blur_buffer2.set_size(resx, resy);
    }

    /// `_updateSelectionCache()`: every `Mesh` or `Sprite` in the selected
    /// objects' subtrees, by id.
    fn selection_cache(&self) -> HashSet<u32> {
        let mut cache = HashSet::new();
        for selected in self.selected_objects.borrow().iter() {
            selected.traverse(&mut |object| {
                let object = object.borrow();
                if object.payload.is_mesh() || object.payload.is_sprite() {
                    cache.insert(object.id);
                }
            });
        }
        cache
    }

    /// One of the two scene renders, under its render-object function.
    fn render_selection(
        &self,
        renderer: &mut Renderer,
        target: &RenderTarget,
        selection: OutlineSelection,
    ) {
        renderer.set_render_target(Some(target.clone()));
        let previous = renderer.outline_selection.replace(selection);
        renderer.render_shared(&self.scene.borrow(), &self.camera);
        renderer.outline_selection = previous;
    }
}

impl NodeUpdate for OutlineState {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `OutlineNode.updateBefore( frame )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        // The output samples the composite even on frames that never draw
        // into it. three.js' `Textures.updateTexture()` gives a render target
        // texture its GPU texture when it is first bound, so an empty
        // selection reads the zeroed 1x1 target; the port creates it here.
        renderer.init_render_target(&self.composite_buffer);

        let selection = self.selection_cache();

        // `RendererUtils.resetRendererState()` — and `restoreRendererState()`
        // at the end — for the state these draws touch.
        let previous_target = renderer.render_target();
        let previous_mrt = renderer.mrt();
        let previous_auto_clear = renderer.auto_clear;
        let previous_clear_color = renderer.clear_color();
        let previous_clear_alpha = renderer.clear_alpha();
        let restore = |renderer: &mut Renderer| {
            renderer.set_render_target(previous_target.clone());
            renderer.set_mrt(previous_mrt.clone());
            renderer.set_clear_color(previous_clear_color, previous_clear_alpha);
            renderer.auto_clear = previous_auto_clear;
        };

        // If no objects are selected, all subsequent passes can be skipped
        // since the outline would be empty anyway. The composite render target
        // is cleared once when transitioning to an empty selection so a
        // previously rendered outline does not linger on screen.
        if selection.is_empty() {
            if self.last_selection_count.get() > 0 {
                renderer.set_mrt(None);
                renderer.auto_clear = true;
                renderer.set_render_target(Some(self.composite_buffer.clone()));
                renderer.set_clear_color(Color::from_hex(0x000000), 0.0);
                renderer.clear(true, self.composite_buffer.depth_buffer());
                restore(renderer);
                self.last_selection_count.set(0);
            }
            return true;
        }

        self.last_selection_count.set(selection.len());

        renderer.set_mrt(None);
        renderer.auto_clear = true;

        let (width, height) = renderer.drawing_buffer_size();
        self.set_size(width, height);

        renderer.set_clear_color(Color::from_hex(0xffffff), 1.0);

        // `reference( 'near' | 'far', 'float', camera )`.
        {
            let camera = self.camera.borrow();
            self.camera_near.set(vec![camera.near()]);
            self.camera_far.set(vec![camera.far()]);
        }

        let selected = Rc::new(selection);

        // 1. Draw non-selected objects in the depth buffer
        self.render_selection(
            renderer,
            &self.depth_buffer,
            OutlineSelection {
                selected: selected.clone(),
                draw_selected: false,
                material: self.depth_material.clone(),
                sprite_material: self.depth_sprite_material.clone(),
            },
        );

        // 2. Draw only the selected objects by comparing the depth buffer of
        // non-selected objects
        self.render_selection(
            renderer,
            &self.mask_buffer,
            OutlineSelection {
                selected,
                draw_selected: true,
                material: self.prepare_mask_material.clone(),
                sprite_material: self.prepare_mask_sprite_material.clone(),
            },
        );

        // 3. Downsample to (at least) half resolution
        renderer.set_render_target(Some(self.mask_down_sample_buffer.clone()));
        renderer.render_quad(&self.copy);

        // 4. Perform edge detection (half resolution)
        renderer.set_render_target(Some(self.edge_buffer1.clone()));
        renderer.render_quad(&self.edge_detection);

        // 5. Apply blur (half resolution)
        renderer.set_render_target(Some(self.blur_buffer1.clone()));
        renderer.render_quad(&self.blur_half[0]);
        renderer.set_render_target(Some(self.edge_buffer1.clone()));
        renderer.render_quad(&self.blur_half[1]);

        // 6. Apply blur (quarter resolution)
        renderer.set_render_target(Some(self.blur_buffer2.clone()));
        renderer.render_quad(&self.blur_quarter[0]);
        renderer.set_render_target(Some(self.edge_buffer2.clone()));
        renderer.render_quad(&self.blur_quarter[1]);

        // 7. Composite
        renderer.set_render_target(Some(self.composite_buffer.clone()));
        renderer.render_quad(&self.composite);

        // restore
        restore(renderer);
        true
    }
}
