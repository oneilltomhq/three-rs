//! Port of `three.js/examples/jsm/tsl/display/PixelationPassNode.js` — a
//! scene pass rendered at `1 / pixelSize` of the drawing buffer with nearest
//! filtering, whose output darkens depth edges and lightens normal edges.
//!
//! Three subclasses `PassNode`; the port wraps one. The pass renders an MRT
//! of `{ output, normal: normalView }` (the §23 machinery), and its three
//! attachments — colour, normal and depth — are all `NearestFilter`, so every
//! tap below is a `textureLoad`.

use crate::cameras::RenderCamera;
use crate::nodes::mrt;
use crate::nodes::node::{TextureSource, Type};
use crate::nodes::tsl::{
    block, dot, float, floor, if_then, int, length, normal_view, output_property, property, sign,
    smoothstep, step, texture_size, texture_uv, to_const, to_var, vec2, vec3, vec4_join,
};
use crate::nodes::NodeRef;
use crate::objects::Scene;
use crate::renderer::{PassNode, PassOptions, Renderer, OUTPUT_ATTACHMENT};
use crate::textures::TextureFilter;

/// `pixelationPass( scene, camera, pixelSize, normalEdgeStrength,
/// depthEdgeStrength )`.
pub struct PixelationPassNode {
    pass: PassNode,
    node: NodeRef,
    /// `this.pixelSize`.
    pixel_size: u32,
}

/// `pixelationPass( scene, camera, pixelSize = 6, normalEdgeStrength = 0.3,
/// depthEdgeStrength = 0.4 )`. The scene and camera are handed to
/// [`PixelationPassNode::render`] instead, as they are for
/// [`PassNode`](crate::renderer::PassNode).
pub fn pixelation_pass(
    pixel_size: u32,
    normal_edge_strength: NodeRef,
    depth_edge_strength: NodeRef,
) -> PixelationPassNode {
    PixelationPassNode::new(pixel_size, normal_edge_strength, depth_edge_strength)
}

impl PixelationPassNode {
    pub fn new(
        pixel_size: u32,
        normal_edge_strength: NodeRef,
        depth_edge_strength: NodeRef,
    ) -> Self {
        // `super( PassNode.COLOR, scene, camera, { minFilter: NearestFilter,
        // magFilter: NearestFilter } )`.
        let pass = PassNode::new_with_options(PassOptions {
            min_filter: TextureFilter::Nearest,
            mag_filter: TextureFilter::Nearest,
            ..PassOptions::default()
        });
        pass.set_size_divisor(pixel_size);

        // `this._mrt = mrt( { output: output, normal: normalView } )`.
        // `normalView` is set up per material, so it is deferred like
        // `webgpu_mrt`'s (`docs/nodes.md` §23).
        let mut scene_mrt = mrt(vec![(OUTPUT_ATTACHMENT, output_property())]);
        scene_mrt.set_deferred("normal", normal_view);
        pass.set_mrt(scene_mrt);

        let node = Self::setup(&pass, normal_edge_strength, depth_edge_strength);

        Self {
            pass,
            node,
            pixel_size,
        }
    }

    /// `PixelationPassNode.setup()`.
    fn setup(
        pass: &PassNode,
        normal_edge_strength: NodeRef,
        depth_edge_strength: NodeRef,
    ) -> NodeRef {
        let texture_node = pass.texture_node(OUTPUT_ATTACHMENT);
        let _ = pass.texture_node("normal");
        let color = pass.texture();
        let normals = pass.texture_named("normal");
        let uv = crate::nodes::tsl::uv();

        let sample_texture = || texture_node.clone();
        let sample_depth = |coord: NodeRef| pass.depth_texture_node_at(coord);
        let sample_normal = |coord: NodeRef| texture_uv(&normals, coord).rgb().normalize();

        let depth_edge_indicator = |depth: &NodeRef,
                                    depth_e: &NodeRef,
                                    depth_w: &NodeRef,
                                    depth_n: &NodeRef,
                                    depth_s: &NodeRef| {
            let diff = property("diff", Type::F32);
            let statements = vec![
                diff.add_assign(depth_e.sub(depth.clone()).clamp(0.0, 1.0)),
                diff.add_assign(depth_w.sub(depth.clone()).clamp(0.0, 1.0)),
                diff.add_assign(depth_n.sub(depth.clone()).clamp(0.0, 1.0)),
                diff.add_assign(depth_s.sub(depth.clone()).clamp(0.0, 1.0)),
            ];
            let value = floor(smoothstep(0.01, 0.02, diff).mul(2.0)).div(2.0);
            (statements, value)
        };

        let neighbor_normal_edge_indicator =
            |x: f64,
             y: f64,
             neighbor_depth: &NodeRef,
             depth: &NodeRef,
             normal: &NodeRef,
             inv_size: &NodeRef| {
                let depth_diff = to_const(None, neighbor_depth.sub(depth.clone()));
                let neighbor_normal = to_const(
                    None,
                    sample_normal(uv.clone().add(vec2(x, y).mul(inv_size.clone()))),
                );

                // Edge pixels should yield to faces who's normals are closer
                // to the bias normal.
                let normal_edge_bias = vec3(1.0, 1.0, 1.0); // This should probably be a parameter.
                let normal_diff = to_const(
                    None,
                    dot(normal.sub(neighbor_normal.clone()), normal_edge_bias),
                );
                let normal_indicator =
                    to_const(None, smoothstep(-0.01, 0.01, normal_diff).clamp(0.0, 1.0));

                // Only the shallower pixel should detect the normal edge.
                let depth_indicator =
                    to_const(None, sign(depth_diff.mul(0.25).add(0.0025)).clamp(0.0, 1.0));

                float(1.0)
                    .sub(dot(normal.clone(), neighbor_normal))
                    .mul(depth_indicator)
                    .mul(normal_indicator)
            };

        let texel = sample_texture();
        let depth = property("depth", Type::F32);
        let normal = property("normal", Type::Vec3);

        let depth_e = to_var(None, float(0.0));
        let depth_w = to_var(None, float(0.0));
        let depth_n = to_var(None, float(0.0));
        let depth_s = to_var(None, float(0.0));

        let inv_size = to_const(
            None,
            vec2(1.0, 1.0)
                .div(texture_size(TextureSource::Texture2D(color), int(0)).to(Type::Vec2)),
        );

        let fetch = if_then(
            depth_edge_strength
                .greater_than(0.0)
                .or(normal_edge_strength.greater_than(0.0)),
            vec![
                depth.assign(sample_depth(uv.clone())),
                normal.assign(sample_normal(uv.clone())),
                depth_e.assign(sample_depth(
                    uv.clone().add(vec2(1.0, 0.0).mul(inv_size.clone())),
                )),
                depth_w.assign(sample_depth(
                    uv.clone().add(vec2(-1.0, 0.0).mul(inv_size.clone())),
                )),
                depth_n.assign(sample_depth(
                    uv.clone().add(vec2(0.0, 1.0).mul(inv_size.clone())),
                )),
                depth_s.assign(sample_depth(
                    uv.clone().add(vec2(0.0, -1.0).mul(inv_size.clone())),
                )),
            ],
        );

        let dei = property("dei", Type::F32);
        let (mut depth_statements, depth_value) =
            depth_edge_indicator(&depth, &depth_e, &depth_w, &depth_n, &depth_s);
        depth_statements.push(dei.assign(depth_value));
        let depth_edges = if_then(depth_edge_strength.greater_than(0.0), depth_statements);

        // `normalEdgeIndicator()`, in the order three adds its four
        // neighbours: south, north, west, east.
        let nei = property("nei", Type::F32);
        let indicator = property("indicator", Type::F32);
        let normal_statements = vec![
            indicator.add_assign(neighbor_normal_edge_indicator(
                0.0, -1.0, &depth_s, &depth, &normal, &inv_size,
            )),
            indicator.add_assign(neighbor_normal_edge_indicator(
                0.0, 1.0, &depth_n, &depth, &normal, &inv_size,
            )),
            indicator.add_assign(neighbor_normal_edge_indicator(
                -1.0, 0.0, &depth_w, &depth, &normal, &inv_size,
            )),
            indicator.add_assign(neighbor_normal_edge_indicator(
                1.0, 0.0, &depth_e, &depth, &normal, &inv_size,
            )),
            nei.assign(step(0.1, indicator)),
        ];
        let normal_edges = if_then(
            normal_edge_strength
                .greater_than(0.0)
                .and(length(normal.clone()).greater_than(0.0)),
            normal_statements,
        );

        let strength = dei.greater_than(0.0).select(
            float(1.0).sub(dei.mul(depth_edge_strength)),
            nei.mul(normal_edge_strength).add(1.0),
        );

        block(
            vec![
                depth_e.clone(),
                depth_w.clone(),
                depth_n.clone(),
                depth_s.clone(),
                inv_size,
                fetch,
                depth_edges,
                normal_edges,
            ],
            vec4_join(vec![texel.clone().mul(strength).rgb(), texel.a()]),
        )
    }

    /// The node for the graph downstream — `RenderPipeline.outputNode`.
    pub fn node(&self) -> NodeRef {
        self.node.clone()
    }

    /// `this.pixelSize`.
    pub fn pixel_size(&self) -> u32 {
        self.pixel_size
    }

    /// `scenePass.pixelSize = n`.
    pub fn set_pixel_size(&mut self, pixel_size: u32) {
        self.pixel_size = pixel_size;
        self.pass.set_size_divisor(pixel_size);
    }

    /// The wrapped `PassNode`.
    pub fn pass(&self) -> &PassNode {
        &self.pass
    }

    /// `PassNode.updateBefore()` at `1 / pixelSize` resolution.
    pub fn render(
        &self,
        renderer: &mut Renderer,
        scene: &mut Scene,
        camera: &mut dyn RenderCamera,
    ) {
        self.pass.render(renderer, scene, camera);
    }
}
