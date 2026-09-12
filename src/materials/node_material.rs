//! Port of `three.js/src/materials/nodes/NodeMaterial.js` — `setup()` and the
//! `setupX()` methods it dispatches to. This is where a material turns into the
//! statements the builder flows into the two shader stages.

use super::MeshBasicNodeMaterial;
use crate::nodes::node::Type;
use crate::nodes::tsl::*;
use crate::nodes::{MaterialFlow, NodeRef};

/// The per-render-object facts three.js reads off `builder.object` and
/// `builder.geometry` during setup.
#[derive(Clone, Copy, Debug, Default)]
pub struct SetupContext {
    /// `Some(count)` when the object is an `InstancedMesh`, which is what makes
    /// `NodeMaterial.setupPosition()` insert the `InstanceNode` transform.
    pub instance_count: Option<usize>,
    /// `InstancedMesh.instanceColor` is present, so `range()` resolves against
    /// the instance index.
    pub instanced: bool,
}

/// `vec4( node )` the way `setupDiffuseColor` builds it: a scalar splats, a
/// `vec3` gains an alpha of 1, a `vec4` passes through.
fn to_vec4(node: NodeRef) -> NodeRef {
    match node.ty() {
        Type::Vec4 => node,
        Type::Vec3 => vec4_join(vec![node, float(1.0)]),
        _ => node.to(Type::Vec4),
    }
}

/// `NodeMaterial.setup()`.
pub fn setup(material: &MeshBasicNodeMaterial, ctx: &SetupContext) -> MaterialFlow {
    let mut pre_vertex = Vec::new();
    let mut fragment = Vec::new();

    // --- setupPosition: the `context.position` stack, flowed into the vertex
    // stage before either stage's own flow. Morphing, skinning and batching
    // plug in here too.
    if let Some(count) = ctx.instance_count {
        let matrix = instance_matrix(count);
        pre_vertex.push(
            position_local().assign(
                matrix
                    .mul(vec4_join(vec![position_local(), float(1.0)]))
                    .xyz(),
            ),
        );
        // `InstanceNode`: the normal is transformed by the inverse transpose of
        // the instance matrix' upper 3x3.
        let m3 = join(
            Type::Mat3,
            vec![
                matrix.element(0).xyz(),
                matrix.element(1).xyz(),
                matrix.element(2).xyz(),
            ],
        );
        let inv_t = transpose(inverse_mat3(m3));
        pre_vertex.push(normal_local().assign(inv_t.mul(normal_local()).normalize()));
    }

    // --- the fragment flow
    let output = if let Some(fragment_node) = &material.fragment_node {
        fragment_node.clone()
    } else {
        // setupDiffuseColor
        let color = match &material.color_node {
            Some(node) => to_vec4(node.clone()),
            None => vec4_join(vec![material_color(), float(1.0)]),
        };
        fragment.push(diffuse_color().assign(color));
        fragment.push(
            diffuse_color()
                .w()
                .assign(diffuse_color().w().mul(material_opacity())),
        );
        // `builder.isOpaque()` — the material is not transparent, blending is
        // NormalBlending and alphaToCoverage is off.
        fragment.push(diffuse_color().w().assign(float(1.0)));

        let outgoing = if let Some(env_map) = &material.env_map {
            // `BasicLightingModel` with an indirect environment contribution.
            fragment.push(indirect_diffuse().assign(vec3(0.0, 0.0, 0.0)));
            fragment.push(indirect_diffuse().assign(
                vec4_join(vec![indirect_diffuse(), float(1.0)])
                    .add(vec4(1.0, 1.0, 1.0, 0.0))
                    .xyz(),
            ));
            fragment.push(ambient_occlusion().assign(float(1.0)));
            fragment
                .push(indirect_diffuse().assign(indirect_diffuse().mul(ambient_occlusion())));
            fragment
                .push(indirect_diffuse().assign(indirect_diffuse().mul(diffuse_color().xyz())));
            fragment.push(direct_diffuse().assign(vec3(0.0, 0.0, 0.0)));
            fragment.push(total_diffuse().assign(direct_diffuse().add(indirect_diffuse())));
            fragment.push(direct_specular().assign(vec3(0.0, 0.0, 0.0)));
            fragment.push(indirect_specular().assign(vec3(0.0, 0.0, 0.0)));
            fragment.push(total_specular().assign(direct_specular().add(indirect_specular())));
            fragment.push(outgoing_light().assign(total_diffuse().add(total_specular())));

            // `MeshBasicNodeMaterial.setupEnvironment()` →
            // `BasicEnvironmentNode( cubeTexture( envMap ) )`, sampled along the
            // reflect vector and blended in by `reflectivity`.
            let dir = material_env_rotation().mul(vec4_join(vec![reflect_vector(), float(1.0)]));
            let env = cube_texture(env_map, dir);
            fragment.push(outgoing_light().assign(mix(
                outgoing_light(),
                outgoing_light().mul(env.xyz()),
                float(1.0).mul(material_reflectivity()),
            )));
            outgoing_light()
        } else {
            // `setupOutgoingLight()` with `lights === false`.
            diffuse_color().xyz()
        };

        // `basicOutput = vec4( outgoingLight, diffuseColor.a ).max( 0 )`.
        vec4_join(vec![outgoing, diffuse_color().w()]).max(float(0.0))
    };

    // --- the vertex flow
    let position = match &material.vertex_node {
        Some(node) => node.clone(),
        None => model_view_projection(),
    };

    MaterialFlow {
        pre_vertex_statements: pre_vertex,
        fragment_statements: fragment,
        output,
        emit_output_property: material.fragment_node.is_none(),
        vertex_statements: Vec::new(),
        position,
    }
}

/// `Background.update()`'s skybox material: the cube map sampled along
/// `normalWorldGeometry` with the background rotation and LOD, on a sphere
/// pinned to the far plane.
pub fn background_color_node(map: &crate::textures::CubeTexture) -> NodeRef {
    let dir = material_env_rotation()
        .mul(background_rotation().mul(vec4_join(vec![normal_world_geometry(), float(1.0)])));
    let sample = cube_texture_level(map, dir, background_blurriness());
    sample.mul(background_intensity())
}

pub fn background_vertex_node() -> NodeRef {
    let is_ortho = camera_projection_matrix()
        .element(3)
        .element(3)
        .equal(float(1.0));
    let ortho_scale = float(1.0)
        .div(camera_projection_matrix().element(1).element(1))
        .mul(float(3.0));
    let modified = is_ortho.select(position_local().mul(ortho_scale), position_local());
    let view_position = model_view_matrix().mul(vec4_join(vec![modified, float(0.0)]));
    let view_proj = camera_projection_matrix().mul(vec4_join(vec![view_position.xyz(), float(1.0)]));
    view_proj.set_z(view_proj.w())
}

/// `QuadMesh.render()`'s temporary `vertexNode`: a full-screen triangle driven
/// entirely by `vertexIndex`.
pub fn quad_vertex_node() -> NodeRef {
    let x = const_array(vec![-1.0, -1.0, 3.0]).element_node(vertex_index());
    let y = const_array(vec![3.0, -1.0, -1.0]).element_node(vertex_index());
    join(Type::Vec4, vec![x, y, float(0.0), float(1.0)])
}

/// `transpose( m )`.
fn transpose(m: NodeRef) -> NodeRef {
    math_call("transpose", m)
}

/// `WGSLNodeBuilder`'s `inverse( mat3 )` polyfill.
fn inverse_mat3(m: NodeRef) -> NodeRef {
    math_call("tsl_inverse_mat3", m)
}

fn math_call(name: &'static str, m: NodeRef) -> NodeRef {
    let ty = m.ty();
    crate::nodes::NodeRef::new(crate::nodes::Node::Math {
        name,
        args: vec![m],
        ty,
    })
}

/// `RangeNode` on an instanced mesh: one `vec4` per instance indexed by a flat
/// `instanceIndex` varying.
pub fn instanced_range(min: crate::math::Color, max: crate::math::Color, count: usize) -> NodeRef {
    let index = to_varying(None, instance_index());
    range(min, max, count, index)
}

/// `Renderer._renderOutput()`'s material: the framebuffer texture sampled at
/// the fragment coordinate, through `renderOutput()`.
pub fn output_fragment_node(framebuffer: &crate::textures::Texture) -> NodeRef {
    let coord = frag_coord().xy().div(viewport_size());
    let color = texture_uv(framebuffer, coord);
    render_output(color)
}

/// `RenderOutputNode.setup()` with `NoToneMapping` and an sRGB output space.
pub fn render_output(color: NodeRef) -> NodeRef {
    let clamped = vec4_join(vec![color.rgb(), color.a().clamp(float(0.0), float(1.0))]);
    let unpremultiplied = unpremultiply_alpha(clamped);
    let encoded = vec4_join(vec![
        srgb_transfer_oetf(unpremultiplied.rgb()),
        unpremultiplied.a(),
    ]);
    premultiply_alpha(encoded)
}
