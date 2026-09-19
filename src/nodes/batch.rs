//! Port of `three.js/src/nodes/accessors/Batch.js` — the vertex-stage batching
//! transform `NodeMaterial.setupPosition()` runs for a `BatchedMesh`, and the
//! `vBatchColor` varying `setupDiffuseColor()` multiplies into the material
//! colour.
//!
//! Three's `batch()` is an unlayouted `Fn`, so its body is inlined into the
//! vertex flow; [`batch`] returns that body as a statement list, the shape the
//! port uses for every inlined accessor (see `morph_reference()`).

use crate::nodes::node::{TextureSource, Type};
use crate::nodes::tsl::*;
use crate::nodes::NodeRef;
use crate::textures::DataTexture;

/// The three data textures a `BatchedMesh` binds. Carried on `SetupContext`,
/// so it is part of the render object's cache key the same way `MorphEntry` is.
#[derive(Clone, Debug)]
pub struct BatchEntry {
    /// `_indirectTexture` — `r32uint`, draw ordinal → instance id.
    pub indirect: DataTexture,
    /// `_matricesTexture` — `rgba32float`, four texels per instance matrix.
    pub matrices: DataTexture,
    /// `_colorsTexture` — `null` until the first `setColorAt()`.
    pub colors: Option<DataTexture>,
}

impl std::hash::Hash for BatchEntry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.indirect.id().hash(state);
        self.matrices.id().hash(state);
        self.colors.as_ref().map(|t| t.id()).hash(state);
    }
}

/// `varyingProperty( 'vec4', 'vBatchColor' )`.
pub fn batch_color() -> NodeRef {
    varying_property("vBatchColor", Type::Vec4, false)
}

/// `varyingProperty( 'uint', 'vBatchIndirectId' )`. Nothing reads it — it is
/// declared because Three declares it, and an integer varying must be flat.
fn batch_indirect_index() -> NodeRef {
    varying_property("vBatchIndirectId", Type::U32, true)
}

/// `int( textureSize( textureLoad( map ), 0 ).x ).toConst()`.
fn texture_width(map: &DataTexture) -> NodeRef {
    to_var(
        None,
        texture_size(TextureSource::Data(map.clone()), int(0))
            .x()
            .to(Type::I32),
    )
}

/// `Batch.js`' `batch( batchMesh )` body.
///
/// The statement list is in Three's *construction* order, not in the order the
/// values are first needed: `createBatchingMatrixNode()` runs its four
/// `toConst()`s before `getBatchingColor()` does, and the port's `to_var()` is
/// lazy, so the vars are pushed as statements to keep the emitted order the
/// same as the dump's.
pub fn batch(entry: &BatchEntry) -> Vec<NodeRef> {
    let mut statements = Vec::new();

    // --- getIndirectIndex( _indirectTexture, int( instanceIndex ) )
    // `builder.getDrawIndex()` is null under WebGPU — `WebGPUBackend` issues one
    // ordinary `drawIndexed()` per sub-range with `firstInstance = i`, so the
    // batching id is `instanceIndex`, the draw ordinal.
    let indirect_size = texture_width(&entry.indirect);
    let ix = to_var(
        None,
        instance_index().to(Type::I32).modulo(indirect_size.clone()),
    );
    let iy = to_var(
        None,
        instance_index().to(Type::I32).div(indirect_size.clone()),
    );
    statements.push(indirect_size);
    statements.push(ix.clone());
    statements.push(iy.clone());

    // `textureLoad( indirectTexture, ivec2( x, y ) ).x` — the `.x` rides along
    // with the fetch, as it does in Three, so the `u32` lands in one property.
    let indirect_id = texture_load_texel(&entry.indirect, ivec2(ix, iy), Type::U32);
    statements.push(batch_indirect_index().assign(indirect_id.clone()));

    // --- createBatchingMatrixNode( _matricesTexture, indirectId )
    let matrices_size = texture_width(&entry.matrices);
    // `float( id ).mul( 4 ).toInt()` — Three goes through `f32` here, and the
    // rounding of that round trip is reproduced rather than replaced by an
    // integer `* 4`.
    let j = to_var(
        None,
        indirect_id.to(Type::F32).mul(float(4.0)).to(Type::I32),
    );
    let mx = to_var(None, j.modulo(matrices_size.clone()));
    let my = to_var(None, j.div(matrices_size.clone()));
    statements.push(matrices_size);
    statements.push(j);
    statements.push(mx.clone());
    statements.push(my.clone());

    // --- getBatchingColor( _colorsTexture, indirectId )
    if let Some(colors) = &entry.colors {
        let colors_size = texture_width(colors);
        let cx = to_var(None, indirect_id.to(Type::I32).modulo(colors_size.clone()));
        let cy = to_var(None, indirect_id.to(Type::I32).div(colors_size.clone()));
        statements.push(colors_size);
        statements.push(cx.clone());
        statements.push(cy.clone());
        let color = texture_load_texel(colors, ivec2(cx, cy), Type::Vec4);
        statements.push(batch_color().assign(color));
    }

    let matrix = join(
        Type::Mat4,
        vec![
            texture_load_texel(&entry.matrices, ivec2(mx.clone(), my.clone()), Type::Vec4),
            texture_load_texel(
                &entry.matrices,
                ivec2(mx.add(int(1)), my.clone()),
                Type::Vec4,
            ),
            texture_load_texel(
                &entry.matrices,
                ivec2(mx.add(int(2)), my.clone()),
                Type::Vec4,
            ),
            texture_load_texel(&entry.matrices, ivec2(mx.add(int(3)), my), Type::Vec4),
        ],
    );

    // `mat3( batchingMatrix )`. WGSL has no `mat3x3<f32>( m4 )` conversion, so
    // the three columns are spelled out — the same shape `InstanceNode` uses.
    let bm = join(
        Type::Mat3,
        vec![
            matrix.element(0).xyz(),
            matrix.element(1).xyz(),
            matrix.element(2).xyz(),
        ],
    );

    statements.push(
        position_local().assign(
            matrix
                .mul(vec4_join(vec![position_local(), float(1.0)]))
                .xyz(),
        ),
    );

    // `normalLocal.div( vec3( bm[0].dot(bm[0]), … ) )` then `bm.mul( … )` —
    // the inverse-transpose of a uniformly-scaled rotation done by hand.
    let transformed = normal_local().div(join(
        Type::Vec3,
        vec![
            bm.element(0).dot(bm.element(0)),
            bm.element(1).dot(bm.element(1)),
            bm.element(2).dot(bm.element(2)),
        ],
    ));
    statements.push(normal_local().assign(bm.mul(transformed).xyz()));

    statements
}
