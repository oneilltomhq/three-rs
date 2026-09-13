//! Port of lib3's `src/sdf-text/BatchedText.js` — steps 4–5 of the ladder.
//!
//! One `InstancedMesh` of unit plane quads, one quad per glyph of every member
//! [`Text`], with four per-instance attributes (`aGlyphUV`, `aGlyphBounds`,
//! `aColor`, `aOpacity`) and the member's `matrixWorld` in the instance matrix.
//! The material is `buildMaterial()`'s TSL chain: `fwidth`-derived
//! anti-aliasing on a bilinear sample of the `R32Float` SDF atlas, with an
//! outline/halo band inside the 0.5 contour.
//!
//! Three things differ from the JS, all forced rather than chosen:
//!
//! 1. **The batch owns its members.** `addText( text )` in the JS keeps a
//!    reference to an object the page also holds; here [`BatchedText::add_text`]
//!    moves the [`Text`] in and [`BatchedText::text_at_mut`] hands it back. The
//!    member's transform is an `Object3D` the batch owns too
//!    ([`BatchedText::member_node`]), since `Text` itself stays the pure layout
//!    type steps 1–3 graded (README deviation 6). `text._batchedText = this` is
//!    therefore *not* set: the opacity write-through exists for a `Text` the
//!    page owns, and a batch that owns its members writes the opacity array
//!    itself — wiring it anyway would be an `Rc` cycle.
//! 2. **The per-instance data travels with the node graph**, as
//!    [`tsl::instanced_data_attribute`], not as a named geometry attribute:
//!    three-rs resolves `attribute( name )` against the geometry's per-*vertex*
//!    map only, and `InstanceBuffer` is the port's existing mechanism for
//!    anything that steps per instance. The consequence is that the four
//!    `needsUpdate = true` flags become one `build_material()` call, which
//!    rebuilds the nodes around fresh copies of the arrays.
//! 3. **`positionNode` is kept**, unlike d33's `LabelBatch` workaround, because
//!    three-rs applies `position_node` *before* the instance matrix (plan §5.2,
//!    `docs/nodes.md` §10) — the order lib3 was written against. On a unit
//!    `PlaneGeometry( 1, 1 )` d33's baked `world × translate × scale` and lib3's
//!    `mix( bounds.x, bounds.z, uv.x )` place every vertex identically, so this
//!    matches d33's pixels as well as lib3's.
//!
//! `Fn( … )()` around `positionNode` and `colorNode` is dropped: it takes no
//! parameters, so `FunctionNode` inlines its body and the emitted WGSL is the
//! same expression either way.

use std::rc::Rc;

use three_rs::core::{Object3D, Object3DNode};
use three_rs::geometries::plane_geometry;
use three_rs::materials::Side;
use three_rs::math::{Color, Matrix4};
use three_rs::nodes::tsl;
use three_rs::nodes::{NodeRef, Type};
use three_rs::objects::InstancedMesh;
use three_rs::textures::Texture;
use three_rs::{MeshBasicNodeMaterial, Node};

use crate::text::Text;
use crate::vector_font::VectorFont;
use crate::vector_font_atlas::VectorFontAtlas;

/// `BatchedText.js:23` — the fraction of each glyph's ink size that the quad
/// *and* the sampled sub-rect are both grown by, so the outline/AA halo around
/// the ink is not clipped at the ink bbox while the mapping stays ink↔ink.
pub const GLYPH_QUAD_PAD: f64 = 0.12;

/// The `options` object of `new BatchedText( maxTextCount, maxGlyphCount,
/// _baseMaterial, options )`, vector mode only — `FontAtlas` is in the skip
/// register, so `options.font != null` is assumed and `_vectorMode` is always
/// true.
#[derive(Clone, Debug)]
pub struct BatchedTextOptions {
    /// `options.outlineWidth ?? 0.03`, in SDF units.
    pub outline_width: f64,
    /// `options.outlineColor`, **linear** — `new THREE.Color( hex )` applies
    /// `SRGBToLinear`, so pass `Color::from_hex( 0xffffff )`. `None` is the
    /// legacy same-colour outline.
    pub outline_color: Option<Color>,
    /// `options.atlasSize ?? 1024`, forwarded to [`VectorFontAtlas::new`].
    pub atlas_size: u32,
}

impl Default for BatchedTextOptions {
    fn default() -> Self {
        Self {
            outline_width: 0.03,
            outline_color: None,
            atlas_size: 1024,
        }
    }
}

/// One member: the `Text` the JS holds in `_members[ id ]`, the `Object3D` it is
/// a subclass of there, and its `_memberGlyphs[ id ]` slice.
struct Member {
    text: Text,
    node: Node,
    glyph_start: usize,
    glyph_count: usize,
}

pub struct BatchedText {
    node: Node,
    /// `this.atlas`.
    pub atlas: VectorFontAtlas,
    font: Option<Rc<VectorFont>>,

    max_text_count: usize,
    max_glyph_count: usize,
    members: Vec<Option<Member>>,
    member_count: usize,
    glyph_count: usize,

    outline_width: f64,
    outline_color: Option<Color>,

    glyph_uv: Vec<f32>,
    glyph_bounds: Vec<f32>,
    colors: Vec<f32>,
    opacities: Vec<f32>,

    /// The `DataTexture` over `atlas.atlas_data()`. Rebuilt rather than mutated
    /// whenever the atlas gains a glyph: the renderer caches GPU textures by
    /// identity, so `needsUpdate` on the old handle would upload nothing.
    texture: Texture,
}

impl BatchedText {
    /// `new BatchedText( maxTextCount = 64, maxGlyphCount = 1024, undefined,
    /// options )`. The constructor ends with `this.count = 0`.
    pub fn new(max_text_count: usize, max_glyph_count: usize, options: BatchedTextOptions) -> Self {
        // `const plane = new THREE.PlaneGeometry( 1, 1 )`.
        let plane = Rc::new(plane_geometry(1.0, 1.0, 1, 1));

        // `const material = new THREE.NodeMaterial()` — the port has no bare
        // `NodeMaterial`; `MeshBasicNodeMaterial` with a `color_node` is the
        // same flow, since `colorNode` replaces the diffuse colour entirely.
        let material = MeshBasicNodeMaterial {
            name: "BatchedText",
            transparent: true,
            depth_write: false,
            side: Side::Double,
            ..Default::default()
        };

        let node = InstancedMesh::new(plane, material, max_glyph_count);
        node.borrow_mut().object_type = "BatchedText";

        let atlas = VectorFontAtlas::new(options.atlas_size);
        let texture = atlas_texture(&atlas);

        let mut batch = Self {
            node,
            atlas,
            font: None,
            max_text_count,
            max_glyph_count,
            members: (0..max_text_count).map(|_| None).collect(),
            member_count: 0,
            glyph_count: 0,
            outline_width: options.outline_width,
            outline_color: options.outline_color,
            glyph_uv: vec![0.0; max_glyph_count * 4],
            glyph_bounds: vec![0.0; max_glyph_count * 4],
            colors: vec![0.0; max_glyph_count * 3],
            opacities: vec![1.0; max_glyph_count],
            texture,
        };

        batch.build_material();
        batch.set_count(0);
        batch
    }

    /// The scene-graph node: `scene.add( batch.node() )`.
    pub fn node(&self) -> &Node {
        &self.node
    }

    /// `InstancedMesh.count` — the glyph quads the renderer draws.
    pub fn count(&self) -> usize {
        self.glyph_count
    }

    /// `get memberCount()`.
    pub fn member_count(&self) -> usize {
        self.member_count
    }

    pub fn max_glyph_count(&self) -> usize {
        self.max_glyph_count
    }

    /// The atlas texture the material samples.
    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    /// The font-loaded continuation of the JS constructor:
    /// `this.vectorFont = font; this.atlas.setFont( font ); … t._needsSync = true`.
    ///
    /// `VectorFontAtlas` here takes the font per call rather than holding it
    /// (README deviation 11), so `setFont`'s whole effect is its reset.
    pub fn set_font(&mut self, font: Rc<VectorFont>) {
        self.font = Some(font);
        self.atlas.reset();
        for member in self.members.iter_mut().flatten() {
            member.text.mark_needs_sync();
        }
    }

    pub fn font(&self) -> Option<&Rc<VectorFont>> {
        self.font.as_ref()
    }

    /// `resetAtlas()`.
    pub fn reset_atlas(&mut self) {
        self.atlas.reset();
        for member in self.members.iter_mut().flatten() {
            member.text.mark_needs_sync();
        }
        self.sync();
    }

    /// `addText( text )` — the member id, or `-1` at capacity.
    pub fn add_text(&mut self, mut text: Text) -> i64 {
        if self.member_count >= self.max_text_count {
            return -1;
        }
        let id = self.member_count;
        self.member_count += 1;

        text.set_vector_mode(true);
        text.set_vector_font(self.font.clone());
        text.mark_needs_sync();

        self.members[id] = Some(Member {
            text,
            node: Object3D::new_node(),
            glyph_start: 0,
            glyph_count: 0,
        });
        id as i64
    }

    /// `removeText( text )`, by id: the slot is cleared, and `_memberCount`
    /// shrinks only when the *last* member went.
    pub fn remove_text(&mut self, member_id: usize) {
        if self.members.get(member_id).map(Option::is_some) != Some(true) {
            return;
        }
        self.members[member_id] = None;
        if member_id + 1 == self.member_count {
            self.member_count -= 1;
        }
    }

    /// `getTextAt( memberId )`.
    pub fn text_at(&self, member_id: usize) -> Option<&Text> {
        self.members.get(member_id)?.as_ref().map(|m| &m.text)
    }

    /// `getTextAt( memberId )`, for mutation. A layout property changed through
    /// this needs a [`BatchedText::sync`] to reach the attributes, as in the JS.
    pub fn text_at_mut(&mut self, member_id: usize) -> Option<&mut Text> {
        self.members
            .get_mut(member_id)?
            .as_mut()
            .map(|m| &mut m.text)
    }

    /// The member's own `Object3D` — `text.position` / `text.quaternion` in the
    /// JS, where `Text extends Object3D`. `sync()` composes its `matrixWorld`.
    pub fn member_node(&self, member_id: usize) -> Option<&Node> {
        self.members.get(member_id)?.as_ref().map(|m| &m.node)
    }

    /// `_memberGlyphs[ memberId ]`.
    pub fn member_glyphs(&self, member_id: usize) -> Option<(usize, usize)> {
        self.members
            .get(member_id)?
            .as_ref()
            .map(|m| (m.glyph_start, m.glyph_count))
    }

    /// `aGlyphUV`, for tests and for callers checking the packing without a GPU.
    pub fn glyph_uv_array(&self) -> &[f32] {
        &self.glyph_uv
    }

    /// `aGlyphBounds`.
    pub fn glyph_bounds_array(&self) -> &[f32] {
        &self.glyph_bounds
    }

    /// `aColor`.
    pub fn color_array(&self) -> &[f32] {
        &self.colors
    }

    /// `aOpacity`.
    pub fn opacity_array(&self) -> &[f32] {
        &self.opacities
    }

    /// `setColorAt( memberId, color )`. The colour is **linear**, as
    /// `THREE.Color` stores it (plan §5.3).
    pub fn set_color_at(&mut self, member_id: usize, color: Color) {
        let Some(Some(member)) = self.members.get_mut(member_id) else {
            return;
        };
        member.text.color = [color.r, color.g, color.b];
        let (start, count) = (member.glyph_start, member.glyph_count);
        if count == 0 {
            return;
        }
        for g in start..start + count {
            self.colors[g * 3] = color.r as f32;
            self.colors[g * 3 + 1] = color.g as f32;
            self.colors[g * 3 + 2] = color.b as f32;
        }
        self.build_material();
    }

    /// `setOpacityAt( memberId, opacity )`. `text._opacity = opacity` is
    /// [`Text::set_opacity`] here: the batch owns the member, so its
    /// write-through sink is `None` and there is no recursion.
    pub fn set_opacity_at(&mut self, member_id: usize, opacity: f64) {
        let Some(Some(member)) = self.members.get_mut(member_id) else {
            return;
        };
        member.text.set_opacity(opacity);
        let (start, count) = (member.glyph_start, member.glyph_count);
        if count == 0 {
            return;
        }
        for g in start..start + count {
            self.opacities[g] = opacity as f32;
        }
        self.build_material();
    }

    /// `get outlineColor()`.
    pub fn outline_color(&self) -> Option<Color> {
        self.outline_color
    }

    /// `set outlineColor( color )` — `None` restores the legacy same-colour
    /// outline by zeroing the mix uniform.
    pub fn set_outline_color(&mut self, color: Option<Color>) {
        self.outline_color = color;
        self.build_material();
    }

    pub fn outline_width(&self) -> f64 {
        self.outline_width
    }

    /// `setMatrixAt( memberId, matrix )` — the member's world matrix when
    /// `memberId` names a registered member, rewriting that member's glyph
    /// matrices; past `_memberCount` it is `InstancedMesh.setMatrixAt`, i.e. one
    /// glyph instance.
    pub fn set_matrix_at(&mut self, member_id: usize, matrix: &Matrix4) {
        let is_member = member_id < self.member_count
            && self.members.get(member_id).map(Option::is_some) == Some(true);
        if !is_member {
            self.node.borrow_mut().set_matrix_at(member_id, matrix);
            return;
        }

        let member = self.members[member_id].as_ref().unwrap();
        member.node.borrow_mut().matrix_world = *matrix;

        let (start, count) = (member.glyph_start, member.glyph_count);
        if member.text.text_render_info().is_none() || count == 0 {
            return;
        }
        self.write_glyph_matrices(matrix, start, count);
    }

    /// `_writeGlyphMatrices( text, info, glyphStart, glyphCount )` — every glyph
    /// of a member gets the member's plain `matrixWorld`; the quad itself comes
    /// from `positionNode`.
    fn write_glyph_matrices(&mut self, world: &Matrix4, start: usize, count: usize) {
        let mut object = self.node.borrow_mut();
        for g in 0..count {
            object.set_matrix_at(start + g, world);
        }
    }

    /// `sync( callback )` — lay every member out, grow the atlas, pack the
    /// attributes. The callback is an idiom for a path that never awaits and is
    /// dropped, as in `Text::sync`.
    pub fn sync(&mut self) {
        // Pass 1: lay out, and collect the distinct non-space glyphs in
        // first-seen order. `new Set()` preserves insertion order and
        // `ensureGlyphs` assigns atlas slots in iteration order, so the order
        // here *is* the slot assignment (plan §5.4).
        let mut chars: Vec<char> = Vec::new();
        let font = self.font.clone();
        for member in self.members.iter_mut().flatten() {
            member.text.set_vector_font(font.clone());
            let info = member.text.sync();
            for glyph in &info.glyphs {
                if glyph.ch != ' ' && !chars.contains(&glyph.ch) {
                    chars.push(glyph.ch);
                }
            }
        }
        let added = self.atlas.ensure_glyphs(font.as_deref(), chars);

        // Pass 2: pack, in member order then glyph order. Member state is
        // copied out first so the atlas can be borrowed mutably below
        // (`get_glyph` memoises, so it takes `&mut self`).
        let mut glyph_index = 0usize;
        let mut truncated_from = None;

        for m in 0..self.member_count {
            let Some(member) = self.members[m].as_ref() else {
                continue;
            };
            let Some(info) = member.text.text_render_info() else {
                continue;
            };

            let glyph_start = glyph_index;
            let glyph_count = info.glyph_count;

            if glyph_index + glyph_count > self.max_glyph_count {
                // `console.warn( '… maxGlyphCount … exceeded; truncating.' )`
                truncated_from = Some(m);
                break;
            }

            let glyphs: Vec<char> = info.glyphs.iter().map(|g| g.ch).collect();
            let bounds = info.glyph_bounds.clone();
            let color = member.text.color;
            let opacity = member.text.opacity();

            {
                let member = self.members[m].as_mut().unwrap();
                member.glyph_start = glyph_start;
                member.glyph_count = glyph_count;
            }
            // `t.updateMatrixWorld()`
            let world = {
                let node = self.members[m].as_ref().unwrap().node.clone();
                node.update_matrix_world(false);
                let world = node.borrow().matrix_world;
                world
            };
            self.write_glyph_matrices(&world, glyph_start, glyph_count);

            for g in 0..glyph_count {
                let gi = glyph_start + g;
                let bi = g * 4;
                let (bx0, by0, bx1, by1) =
                    (bounds[bi], bounds[bi + 1], bounds[bi + 2], bounds[bi + 3]);

                if glyphs[g] != ' ' {
                    let metrics = self.atlas.get_glyph(font.as_deref(), glyphs[g]);

                    // `uvRect.viewBox ?? [ 0, 0, 1, 1 ]`, with a degenerate box
                    // falling back to the whole tile.
                    let vb = metrics.view_box;
                    let vb = if vb[2] > vb[0] && vb[3] > vb[1] {
                        vb
                    } else {
                        [0.0, 0.0, 1.0, 1.0]
                    };

                    // Grow the quad and the sampled sub-rect by the same
                    // fraction of their own size, so the mapping stays ink↔ink
                    // while the SDF margin the halo lives in is revealed.
                    let pad = GLYPH_QUAD_PAD as f32;
                    let bw = bx1 - bx0;
                    let bh = by1 - by0;
                    self.glyph_bounds[gi * 4] = bx0 - bw * pad;
                    self.glyph_bounds[gi * 4 + 1] = by0 - bh * pad;
                    self.glyph_bounds[gi * 4 + 2] = bx1 + bw * pad;
                    self.glyph_bounds[gi * 4 + 3] = by1 + bh * pad;

                    let pad = GLYPH_QUAD_PAD;
                    let vbw = vb[2] - vb[0];
                    let vbh = vb[3] - vb[1];
                    let su0 = (vb[0] - vbw * pad).max(0.0);
                    let sv0 = (vb[1] - vbh * pad).max(0.0);
                    let su1 = (vb[2] + vbw * pad).min(1.0);
                    let sv1 = (vb[3] + vbh * pad).min(1.0);

                    self.glyph_uv[gi * 4] = (metrics.u + su0 * metrics.w) as f32;
                    self.glyph_uv[gi * 4 + 1] = (metrics.v + sv0 * metrics.h) as f32;
                    self.glyph_uv[gi * 4 + 2] = ((su1 - su0) * metrics.w) as f32;
                    self.glyph_uv[gi * 4 + 3] = ((sv1 - sv0) * metrics.h) as f32;
                } else {
                    // A blank glyph keeps its unpadded bounds and gets a zero UV
                    // rect, but still consumes an instance: `aGlyphUV.z == 0` is
                    // what the shader's `isBlank` tests, and the instance counts
                    // have to match the JS or the painter order shifts (§5.4).
                    self.glyph_bounds[gi * 4] = bx0;
                    self.glyph_bounds[gi * 4 + 1] = by0;
                    self.glyph_bounds[gi * 4 + 2] = bx1;
                    self.glyph_bounds[gi * 4 + 3] = by1;

                    self.glyph_uv[gi * 4] = 0.0;
                    self.glyph_uv[gi * 4 + 1] = 0.0;
                    self.glyph_uv[gi * 4 + 2] = 0.0;
                    self.glyph_uv[gi * 4 + 3] = 0.0;
                }

                self.colors[gi * 3] = color[0] as f32;
                self.colors[gi * 3 + 1] = color[1] as f32;
                self.colors[gi * 3 + 2] = color[2] as f32;
                self.opacities[gi] = opacity as f32;

                glyph_index += 1;
            }
        }

        if let Some(m) = truncated_from {
            eprintln!(
                "BatchedText: maxGlyphCount ({}) exceeded; truncating at member {m}.",
                self.max_glyph_count
            );
        }

        self.glyph_count = glyph_index;
        self.set_count(glyph_index);

        if added {
            self.texture = atlas_texture(&self.atlas);
        }

        // The four `needsUpdate = true` flags: the nodes holding the arrays are
        // rebuilt around fresh copies.
        self.build_material();
    }

    /// `this.count = n`.
    ///
    /// `instanceMatrix` keeps its full `maxGlyphCount` length: `InstanceNode`
    /// sizes the uniform-or-attribute decision on `instanceMatrix.count`, not on
    /// the draw count, so trimming the array would flip that branch per frame.
    fn set_count(&mut self, count: usize) {
        let mut object = self.node.borrow_mut();
        if let Some(instanced) = object.payload.instanced_mesh_mut() {
            instanced.count = count;
        }
    }

    /// `buildMaterial( material )`.
    fn build_material(&mut self) {
        let uv_data = Rc::new(self.glyph_uv.clone());
        let bounds_data = Rc::new(self.glyph_bounds.clone());
        let color_data = Rc::new(self.colors.clone());
        let opacity_data = Rc::new(self.opacities.clone());

        // `attribute( 'aGlyphUV', 'vec4' )` and friends.
        let a_glyph_uv = tsl::instanced_data_attribute(&uv_data, 4, 0, Type::Vec4);
        let a_glyph_bounds = tsl::instanced_data_attribute(&bounds_data, 4, 0, Type::Vec4);
        let a_color = tsl::instanced_data_attribute(&color_data, 3, 0, Type::Vec3);
        let a_opacity = tsl::instanced_data_attribute(&opacity_data, 1, 0, Type::F32);

        let outline_width = tsl::uniform_value(Type::F32, vec![self.outline_width]);
        let halo = self.outline_color.unwrap_or(Color::new(0.0, 0.0, 0.0));
        let outline_color_uniform = tsl::uniform_value(Type::Vec3, vec![halo.r, halo.g, halo.b]);
        let outline_color_mix = tsl::uniform_value(
            Type::F32,
            vec![if self.outline_color.is_some() { 1.0 } else { 0.0 }],
        );

        let quad_uv = tsl::uv();

        // positionNode = vec3( mix( b.x, b.z, st.x ), mix( b.y, b.w, st.y ), 0 )
        let position_node = tsl::join(
            Type::Vec3,
            vec![
                tsl::mix(a_glyph_bounds.x(), a_glyph_bounds.z(), quad_uv.x()),
                tsl::mix(a_glyph_bounds.y(), a_glyph_bounds.w(), quad_uv.y()),
                tsl::float(0.0),
            ],
        );

        // The JS builds a `vec4` and swizzles `.xy`; the two zero lanes are
        // dead, so only the `vec2` is built here. The atlas is Y-down, hence the
        // `1 - uv.y`.
        let atlas_uv = tsl::join(
            Type::Vec2,
            vec![
                a_glyph_uv.x().add(quad_uv.x().mul(a_glyph_uv.z())),
                a_glyph_uv
                    .y()
                    .add(tsl::float(1.0).sub(quad_uv.y()).mul(a_glyph_uv.w())),
            ],
        );

        let sdf_value = tsl::texture_uv(&self.texture, atlas_uv).x();
        let edge = tsl::float(0.5);
        let aa_width = tsl::fwidth(sdf_value.clone()).mul(0.5);

        let fill_alpha = tsl::smoothstep(
            edge.sub(aa_width.clone()),
            edge.add(aa_width.clone()),
            sdf_value.clone(),
        );

        let outline_edge = edge.sub(outline_width);
        let outline_alpha = tsl::smoothstep(
            outline_edge.sub(aa_width.clone()),
            outline_edge.add(aa_width),
            sdf_value,
        );
        let outline_only = tsl::max(outline_alpha.sub(fill_alpha.clone()), tsl::float(0.0));

        let is_blank = a_glyph_uv.z().equal(tsl::float(0.0));
        let alpha = is_blank
            .select(tsl::float(0.0), tsl::max(fill_alpha.clone(), outline_only))
            .mul(a_opacity);

        // Outline pixels take the halo colour (when set); fill pixels keep the
        // per-member colour, with the SDF fill coverage blending the boundary.
        let halo_rgb = tsl::mix(a_color.clone(), outline_color_uniform, outline_color_mix);
        let rgb = tsl::mix(halo_rgb, a_color, fill_alpha);

        let color_node: NodeRef = tsl::join(Type::Vec4, vec![rgb, alpha]);

        let mut object = self.node.borrow_mut();
        let material = object
            .payload
            .mesh_mut()
            .and_then(|mesh| mesh.material.as_mut())
            .expect("sdf-text: BatchedText always has a material");
        material.position_node = Some(position_node);
        material.color_node = Some(color_node);
    }
}

/// `new THREE.DataTexture( atlasData, atlasSize, atlasSize, RedFormat,
/// FloatType )` with `minFilter = magFilter = LinearFilter` — the bilinear
/// filtering is load-bearing (it is what makes the SDF a *field* rather than a
/// staircase), and the texture must not be tagged sRGB (plan §5.3).
fn atlas_texture(atlas: &VectorFontAtlas) -> Texture {
    Texture::data_r32float(atlas.atlas_size(), atlas.atlas_size(), atlas.atlas_data())
}
