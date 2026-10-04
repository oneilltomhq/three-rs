//! Port of `three.js/examples/jsm/tsl/display/RetroPassNode.js` — a scene
//! pass drawn the way a PlayStation 1 drew it: at a quarter of the canvas,
//! with nearest filtering, with every vertex snapped to that low-resolution
//! grid, with no texture mipmaps and, optionally, with affine (not
//! perspective-correct) texture mapping.

use std::cell::{Cell, OnceCell};
use std::rc::Rc;

use crate::materials::{MaterialKind, MeshBasicNodeMaterial};
use crate::math::Color;
use crate::nodes::tsl::{
    block, camera_projection_matrix, camera_view_matrix, custom, float, mix, position_world,
    screen_size, texture_uv, to_varying, uv, vec2, vec4_join,
};
use crate::nodes::{ContextValue, CustomNode, NodeBuilder, NodeRef, Type};
use crate::renderer::{CameraRef, PassNode, PassOptions, RenderObjectFunction, SceneRef};
use crate::textures::TextureFilter;

/// `retroPass( scene, camera, options )`'s options object.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct RetroPassOptions {
    /// `options.affineDistortion` — when set, every 2D texture tap of a
    /// classic material reads `mix( uv(), affineUv / w, affineDistortion )`:
    /// 0 is perspective-correct, 1 the PS1's affine mapping. `null` by
    /// default, which leaves the uv alone.
    pub affine_distortion: Option<NodeRef>,
    /// `options.filterTextures` — `false` by default, which samples every
    /// texture of a classic material at level 0 (`getTextureLevel: () =>
    /// uint( 0 )`), so no mipmaps.
    pub filter_textures: bool,
}

impl RetroPassOptions {
    /// The options with `affine_distortion` set.
    pub fn affine_distortion(mut self, node: impl Into<NodeRef>) -> Self {
        self.affine_distortion = Some(node.into());
        self
    }

    /// The options with `filter_textures` set.
    pub fn filter_textures(mut self, filter_textures: bool) -> Self {
        self.filter_textures = filter_textures;
        self
    }
}

/// `RetroPassNode`, a port of
/// `three.js/examples/jsm/tsl/display/RetroPassNode.js`: a scene pass drawn
/// the way a PlayStation 1 drew it. It renders at a quarter of the canvas,
/// with nearest filtering, with every vertex snapped to that low-resolution
/// grid, with no texture mipmaps and, optionally, with affine (not
/// perspective-correct) texture mapping.
///
/// Three subclasses `PassNode` and swaps the renderer's render-object
/// function around `super.updateBefore()`. The port wraps a [`PassNode`] and
/// gives it a render-object function (the crate-private
/// `RenderObjectFunction`). The pass installs it on the renderer for the
/// length of its own render, and the render loop asks it once per draw,
/// exactly where three calls the function.
/// [`ToonOutlinePassNode`](super::ToonOutlinePassNode) swaps the same
/// function in three; its port predates this hook and has a dedicated
/// `toon_outline` field on the renderer instead.
///
/// # Which material a draw is made with
///
/// Three derives a retro material per source material (cached on its
/// `version`): a `MeshBasicNodeMaterial` for a basic source, a
/// `MeshPhongNodeMaterial` for anything else, whose `vertexNode` is the
/// snapping `_clipSpaceRetro`, whose `colorNode` is `materialColor` and whose
/// `contextNode` carries the `getUV` / `getTextureLevel` overrides. It then
/// copies **every property of the source that the retro material also has**
/// over it (`for ( const property in material )`). A classic material
/// (`MeshStandardMaterial`, `MeshBasicMaterial`, what `GLTFLoader` makes) has
/// none of the node slots, so its maps, colours and flags come across and the
/// retro nodes stay. A node material has all of them, null ones included, so
/// its own `vertexNode`, `colorNode`, `contextNode` and the rest overwrite the
/// retro ones and it is drawn as itself, as a basic or a Phong material: the
/// smoke in `webgpu_postprocessing_retro` is neither snapped nor level-0
/// sampled in three's dump (`m05`/`m06`), and the mug is (`m03`/`m04`).
///
/// The port has one material struct for both, so "classic" is decided by the
/// node slots: a source with none of them set is a classic one. See
/// `is_node_material`.
///
/// # Not ported
///
/// - the `MeshStandardMaterial` + `envMap` / `scene.environment` branch,
///   which mixes a `CubeMapNode` reflection into `colorNode` by metalness:
///   the page takes it only for the Damaged Helmet, which is not its default
///   model. A standard source is drawn as a Phong one without it.
/// - the background draw keeps its own material. Three's background is a
///   plain `NodeMaterial`, so its retro material is a Phong one with
///   `lights = false` and every node copied over; that draws diffuse plus a
///   black emissive, which is the background's colour unchanged (`m01`).
/// - sprite, points, fat-line and normal materials are drawn as themselves;
///   three would hand them to a Phong material, which the port's flows for
///   those kinds cannot be.
/// - the per-material cache and `dispose()`: the port derives the material
///   per draw (a clone) and keys its programs on the source's, so a changed
///   source or a changed [`set_filter_textures`](RetroPassNode::set_filter_textures)
///   is picked up on the next frame without one.
pub struct RetroPassNode {
    pass: PassNode,
    node: NodeRef,
    function: Rc<RetroFunction>,
}

/// `retroPass( scene, camera, options )`.
pub fn retro_pass(scene: SceneRef, camera: CameraRef, options: RetroPassOptions) -> RetroPassNode {
    RetroPassNode::new(scene, camera, options)
}

impl RetroPassNode {
    /// `new RetroPassNode( scene, camera, options )`: a colour pass at a
    /// resolution scale of 0.25 with `NearestFilter` on both sides.
    ///
    /// Three also sets the target's type to `UnsignedByteType`, and
    /// `PassNode.setup()` then overwrites it with the renderer's output
    /// buffer type, `HalfFloatType` — its dump binds an `rgba16float` target —
    /// so the port's pass keeps `HalfFloat`.
    pub fn new(scene: SceneRef, camera: CameraRef, options: RetroPassOptions) -> Self {
        let pass = PassNode::new_with_options(PassOptions {
            min_filter: TextureFilter::Nearest,
            mag_filter: TextureFilter::Nearest,
            ..PassOptions::default()
        });
        pass.set_scene(scene, camera);
        pass.set_resolution_scale(0.25);

        let function = Rc::new(RetroFunction {
            affine_distortion: options.affine_distortion,
            filter_textures: Cell::new(options.filter_textures),
            id: next_id(),
        });
        pass.set_render_object_function(Some(function.clone()));

        let node = custom(RetroTextureNode { pass: pass.clone() });

        Self {
            pass,
            node,
            function,
        }
    }

    /// `getTextureNode()` — the pass's colour, to compose with. Its uv is the
    /// `getUV` of the [`context`](crate::nodes::tsl::context) it is built in,
    /// as a `PassTextureNode`'s is, so
    /// [`replace_default_uv`](crate::nodes::tsl::replace_default_uv) around
    /// it reads the pass where the given uv says (`webgpu_postprocessing_retro`
    /// reads it through [`barrel_uv`](super::barrel_uv)). Without one it is
    /// `uv()`.
    pub fn node(&self) -> NodeRef {
        self.node.clone()
    }

    /// The wrapped pass.
    pub fn pass(&self) -> &PassNode {
        &self.pass
    }

    /// `retroPass.filterTextures = value` (followed, on the page, by
    /// `retro.dispose()`): whether classic materials' textures keep their
    /// mipmaps. Takes effect on the next frame.
    pub fn set_filter_textures(&self, filter_textures: bool) {
        self.function.filter_textures.set(filter_textures);
    }

    /// `retroPass.filterTextures`.
    pub fn filter_textures(&self) -> bool {
        self.function.filter_textures.get()
    }

    /// `retroPass.affineDistortionNode`.
    pub fn affine_distortion(&self) -> Option<NodeRef> {
        self.function.affine_distortion.clone()
    }
}

impl From<&RetroPassNode> for NodeRef {
    fn from(retro: &RetroPassNode) -> NodeRef {
        retro.node()
    }
}

fn next_id() -> usize {
    thread_local! { static NEXT: Cell<usize> = const { Cell::new(0) }; }
    NEXT.with(|n| {
        let id = n.get();
        n.set(id + 1);
        id
    })
}

/// The pass's `PassTextureNode`: a tap on its colour texture whose uv is
/// `builder.context.getUV` when there is one — `TextureNode.setup()`'s
/// `if ( uvNode === null && builder.context.getUV ) uvNode =
/// builder.context.getUV( this, builder )` — and `uv()` otherwise. The texture
/// is unfilterable (`NearestFilter` on both sides), so the tap is a
/// `textureLoad`, as three's dump has it.
///
/// It holds the pass, as three's `PassTextureNode.passNode` does, so the pass
/// is rendered for as long as a graph reads it.
struct RetroTextureNode {
    pass: PassNode,
}

impl CustomNode for RetroTextureNode {
    fn type_name(&self) -> &'static str {
        "PassTextureNode"
    }

    fn node_type(&self) -> Type {
        Type::Vec4
    }

    fn setup(&self, builder: &NodeBuilder) -> NodeRef {
        let coord = builder.context("getUV").unwrap_or_else(uv);
        texture_uv(&self.pass.texture(), coord)
    }
}

/// `_affineUv`, `_w` and `_clipSpaceRetro`: module-level in three, so one
/// node per thread here.
struct ClipSpaceRetro {
    affine_uv: NodeRef,
    w: NodeRef,
    position: NodeRef,
}

fn clip_space_retro() -> Rc<ClipSpaceRetro> {
    thread_local! { static CELL: OnceCell<Rc<ClipSpaceRetro>> = const { OnceCell::new() }; }
    CELL.with(|c| {
        c.get_or_init(|| {
            // `varying( vec2() )` / `varying( float() )`.
            let affine_uv = to_varying(None, vec2(0.0, 0.0));
            let w = to_varying(None, float(0.0));

            let default_position = camera_projection_matrix()
                .mul(camera_view_matrix())
                .mul(position_world());

            let half_w = default_position.w().mul(2.0);
            let rounded_position = default_position
                .xy()
                .div(half_w.clone())
                .mul(screen_size().xy())
                .round()
                .div(screen_size().xy())
                .mul(half_w);

            let assign_affine_uv = affine_uv.assign(uv().mul(default_position.w()));
            let assign_w = w.assign(default_position.w());

            let position = block(
                vec![assign_affine_uv, assign_w],
                vec4_join(vec![rounded_position.xy(), default_position.zw()]),
            );

            Rc::new(ClipSpaceRetro {
                affine_uv,
                w,
                position,
            })
        })
        .clone()
    })
}

/// Whether `material` is what three calls a node material: one with any node
/// slot set. A classic three material has none of `NodeMaterial`'s
/// properties, so `RetroPassNode`'s property copy leaves the retro nodes in
/// place; a node material overwrites them. `GLTFLoader` sets none of these.
pub(crate) fn is_node_material(material: &MeshBasicNodeMaterial) -> bool {
    let m = material;
    [
        &m.color_node,
        &m.opacity_node,
        &m.alpha_test_node,
        &m.emissive_node,
        &m.mask_node,
        &m.cast_shadow_node,
        &m.received_shadow_position_node,
        &m.specular_node,
        &m.normal_node,
        &m.position_node,
        &m.scale_node,
        &m.rotation_node,
        &m.size_node,
        &m.vertex_node,
        &m.fragment_node,
        &m.output_node,
        &m.backdrop_node,
        &m.backdrop_alpha_node,
        &m.metalness_node,
        &m.roughness_node,
        &m.depth_node,
    ]
    .iter()
    .any(|slot| slot.is_some())
        || m.mrt_node.is_some()
        || m.context_overrides.is_some()
        || m.context_node.is_some()
        || m.lighting_model.is_some()
        || m.lights_node.is_some()
}

/// The render-object function `RetroPassNode.updateBefore()` installs.
struct RetroFunction {
    affine_distortion: Option<NodeRef>,
    filter_textures: Cell<bool>,
    id: usize,
}

impl RenderObjectFunction for RetroFunction {
    fn material(&self, source: &MeshBasicNodeMaterial, background: bool) -> MeshBasicNodeMaterial {
        let mut material = source.clone();
        let basic = match source.kind {
            MaterialKind::Basic => true,
            MaterialKind::Phong
            | MaterialKind::Lambert
            | MaterialKind::Standard
            | MaterialKind::Physical
            | MaterialKind::Toon => false,
            // Not ported; see the module documentation.
            _ => return material,
        };
        if background {
            return material;
        }

        // `new MeshBasicNodeMaterial()` / `new MeshPhongNodeMaterial()`, with
        // the source's properties copied over. A Phong material's own
        // `specular` and `shininess` are the only ones a non-Phong source has
        // nothing to copy into.
        if !basic {
            if source.kind != MaterialKind::Phong {
                let phong = MeshBasicNodeMaterial::phong(Color::new(1.0, 1.0, 1.0));
                material.specular = phong.specular;
                material.shininess = phong.shininess;
            }
            material.kind = MaterialKind::Phong;
            material.lights = source.lights || !is_node_material(source);
        }

        if is_node_material(source) {
            // Every node slot, null ones included, was copied over the retro
            // material's: the source is drawn as itself.
            return material;
        }

        // `retroMaterial.vertexNode = material.vertexNode || _clipSpaceRetro`;
        // `colorNode = material.colorNode || materialColor`, which is the
        // port's `color_node: None`.
        let clip = clip_space_retro();
        material.vertex_node = Some(clip.position.clone());
        material.color_node = None;

        let mut context = ContextValue::new();
        if let Some(affine) = &self.affine_distortion {
            // `getUV: ( texture ) => texture.isCubeTextureNode ? reflectVector
            // : affine.mix( uv(), _affineUv.div( _w ) )`. Only 2D taps read
            // `getUV` in the port, and a cube tap's default is
            // `reflectVector` already.
            context = context.set(
                "getUV",
                mix(uv(), clip.affine_uv.div(clip.w.clone()), affine.clone()),
            );
        }
        if !self.filter_textures.get() {
            // `getTextureLevel: () => uint( 0 )`, which the builder formats
            // as the `f32` level `textureSampleLevel` takes: `0.0` in the dump.
            context = context.set("getTextureLevel", float(0.0));
        }
        material.context_node = Some(context);

        material
    }

    fn variant(&self) -> u64 {
        (self.id as u64) << 1 | u64::from(self.filter_textures.get())
    }
}
