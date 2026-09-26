//! `renderer.info` (three.js/src/renderers/common/Info.js): what the last
//! frame drew, what it had to build, and what the renderer is still holding.
//!
//! Time is the proof a frame is cheap but not the diagnosis (issue #57): it
//! jitters, so its ceiling has to be generous, and a regression that re-uploads
//! every geometry of a small scene stays under it. These are counts, so they
//! are exact and a steady frame's build block is asserted at zero rather than
//! under a bound (issue #67).

use wgpu::PrimitiveTopology;

/// What one frame drew and what it cost to get there.
///
/// three.js resets `render` and `memory`-adjacent per-frame fields at the top
/// of `Renderer.render()` and exposes `info.autoReset` for the composed frame,
/// where one application frame is several `render()` calls; this port does the
/// same. See [`Info::auto_reset`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Info {
    /// `info.autoReset`: reset [`render`](Self::render) and
    /// [`build`](Self::build) at the top of every [`Renderer::render`].
    ///
    /// A rung whose frame is several renders — a pass into a render target and
    /// then a full-screen quad (rtt, depth texture), three `PassNode`s and a
    /// `RenderPipeline` (postprocessing masking) — wants the counts for the
    /// whole frame, so it turns this off and calls [`Info::reset`] itself, the
    /// way three.js' postprocessing does. `three_rs::testing::strip` is the
    /// only caller in the tree.
    ///
    /// [`Renderer::render`]: crate::Renderer::render
    pub auto_reset: bool,

    /// Reset every frame: what was drawn.
    pub render: RenderCounts,
    /// Reset every frame: what the frame had to build or upload. Every field
    /// is zero on a frame whose scene the renderer has already seen.
    pub build: BuildCounts,
    /// Not reset: what the renderer is holding on to right now.
    pub memory: MemoryCounts,
    /// Not reset: `info.compute`.
    pub compute: ComputeCounts,
}

/// `info.compute`.
///
/// three.js has a second field, `frameCalls`, cleared by `Info.reset()` — which
/// its animation loop calls at the *top of the frame callback*
/// (`Animation.js:81`), before `renderer.compute()` runs. This port has no
/// animation loop and resets in [`Renderer::render`], which runs *after* the
/// frame's compute calls, so a `frame_calls` here would always read zero. The
/// cumulative count is the honest one; the per-frame question "did this frame
/// build a compute pipeline" is answered by [`BuildCounts`] instead.
///
/// [`Renderer::render`]: crate::Renderer::render
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ComputeCounts {
    /// `info.compute.calls`: `Renderer::compute()` calls since startup,
    /// `onInit`'s recursive call included — three counts it too.
    pub calls: u64,
}

/// `info.render`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderCounts {
    /// Draw calls (`drawArrays` / `drawElements`; here `draw` /
    /// `draw_indexed`), instanced draws counting once.
    pub calls: u64,
    /// Primitives, with the instance count multiplied in, as three.js counts
    /// them.
    pub triangles: u64,
    pub points: u64,
    pub lines: u64,
}

/// The per-frame build block: the work a steady frame must not do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BuildCounts {
    /// `NodeBuilder::build` calls — a material's WGSL generated, and compiled
    /// into a [`programs`](MemoryCounts::programs) entry if that WGSL is new.
    pub programs_compiled: u64,
    /// Render pipelines created for a program and a pass state.
    pub pipelines_built: u64,
    /// Geometries uploaded to the GPU (`Geometries.get()`'s misses).
    pub geometries_uploaded: u64,
    /// Attribute and index buffers written as part of those uploads — four for
    /// a geometry with position, normal, uv and an index. Per-draw data that
    /// the scene changes every frame (an `InstancedMesh`'s matrix, morph
    /// influences) is written into a buffer the draw already has, which is
    /// not a build and is not counted here or anywhere.
    pub buffers_written: u64,
    /// Textures uploaded: 2D, cube and the morph data-array textures.
    pub textures_uploaded: u64,
    /// GPU buffers created for bindings: a draw's uniform groups, bone
    /// matrices, morph influences and instance data the first time it is
    /// drawn, a `range()` / `uniformArray()` / instanced-attribute buffer, an
    /// `instancedArray()`'s storage (issue #137).
    pub buffers_created: u64,
    /// Texture views created for bindings — one per texture and view
    /// dimension, and again when a render target is re-allocated.
    pub views_created: u64,
    /// Samplers created — one per distinct filter / wrap / compare
    /// combination for the renderer's life.
    pub samplers_created: u64,
    /// Bind groups created — one per draw group the first time it is drawn,
    /// and again when one of its resources is replaced.
    pub bind_groups_created: u64,
}

/// `info.memory` plus `info.programs.length`: resident counts, never reset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemoryCounts {
    /// Entries in the uploaded-geometry cache, swept as the consumer drops
    /// geometries (issue #58).
    pub geometries: usize,
    /// Entries in the 2D and cube texture caches, swept as the consumer drops
    /// textures (issue #158). A render target's own colour texture and a morph
    /// data-array texture live on the texture rather than in a renderer map,
    /// so they upload but do not land here; they are freed with their owner.
    pub textures: usize,
    /// Distinct compiled programs — `renderer.info.programs.length`. Two
    /// materials that generate the same WGSL share one.
    pub programs: usize,
}

impl Info {
    pub(crate) fn new() -> Self {
        Self {
            auto_reset: true,
            ..Self::default()
        }
    }

    /// `info.reset()`: clear the two per-frame blocks, leaving `memory`.
    pub fn reset(&mut self) {
        self.render = RenderCounts::default();
        self.build = BuildCounts::default();
    }

    /// One `draw` / `draw_indexed`: `elements` vertices or indices, drawn
    /// `instances` times.
    pub(crate) fn record_draw(
        &mut self,
        topology: PrimitiveTopology,
        elements: u32,
        instances: u32,
    ) {
        self.render.calls += 1;

        let (elements, instances) = (u64::from(elements), u64::from(instances));
        match topology {
            PrimitiveTopology::TriangleList => self.render.triangles += elements / 3 * instances,
            PrimitiveTopology::TriangleStrip => {
                self.render.triangles += elements.saturating_sub(2) * instances
            }
            PrimitiveTopology::LineList => self.render.lines += elements / 2 * instances,
            PrimitiveTopology::LineStrip => {
                self.render.lines += elements.saturating_sub(1) * instances
            }
            PrimitiveTopology::PointList => self.render.points += elements * instances,
        }
    }
}

impl BuildCounts {
    /// Everything this frame built or uploaded. Zero is the steady frame.
    pub fn total(&self) -> u64 {
        self.programs_compiled
            + self.pipelines_built
            + self.geometries_uploaded
            + self.buffers_written
            + self.textures_uploaded
            + self.buffers_created
            + self.views_created
            + self.samplers_created
            + self.bind_groups_created
    }
}

impl std::fmt::Display for Info {
    /// One line, as the e2e harness and the film strip's gutter print it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "calls {} triangles {} lines {} | programs {} pipelines {} geometries {} \
             buffers {} textures {} | created {} buffers {} views {} samplers {} bind groups \
             | resident {} geometries {} textures {} programs",
            self.render.calls,
            self.render.triangles,
            self.render.lines,
            self.build.programs_compiled,
            self.build.pipelines_built,
            self.build.geometries_uploaded,
            self.build.buffers_written,
            self.build.textures_uploaded,
            self.build.buffers_created,
            self.build.views_created,
            self.build.samplers_created,
            self.build.bind_groups_created,
            self.memory.geometries,
            self.memory.textures,
            self.memory.programs,
        )
    }
}
