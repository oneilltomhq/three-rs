//! Occlusion queries: `object.occlusionTest` and `renderer.isOccluded()`
//! (`WebGPUBackend.beginRender()` / `draw()` / `finishRender()` /
//! `resolveOccludedAsync()`). `docs/nodes.md` §36.
//!
//! A render context whose pass draws an object with `occlusion_test` set gets
//! an occlusion query set with one query per run of consecutive draws of such
//! an object. After the pass the set is resolved and copied into a mappable
//! buffer, and that buffer is mapped one frame later — three's
//! `currentOcclusionQueryBuffer` is the *previous* `finishRender()`'s buffer —
//! so an answer reaches `isOccluded()` two frames after the draw it measured,
//! at the earliest. Frame one, the graded one, never has an answer.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

/// `renderContext`, as far as occlusion is concerned: the scene's id and the
/// render target's texture id (`None` for the canvas).
///
/// three's `RenderContexts.get( scene, camera, renderTarget, mrt, callDepth )`
/// also keys on the camera. The port's cameras have no shared identity to key
/// on (`RenderCamera` is a trait over several owned structs), so two cameras
/// rendering one scene into one target share a context here. No page on the
/// ladder does that with an occlusion test.
pub(super) type ContextKey = (u32, Option<usize>);

/// The map's states: waiting on `mapAsync`, mapped, failed.
const WAITING: u8 = 0;
const MAPPED: u8 = 1;
const FAILED: u8 = 2;

/// A read-back buffer and the objects its queries measured, in query order —
/// `occlusionQueryBuffer` with `occlusionQueryObjects`.
struct Readback {
    buffer: wgpu::Buffer,
    objects: Vec<u32>,
}

/// A [`Readback`] whose `mapAsync()` has been issued.
struct Pending {
    readback: Readback,
    state: Arc<AtomicU8>,
}

/// `renderContextData`'s occlusion fields.
#[derive(Default)]
pub(super) struct OcclusionContext {
    /// `occlusionQuerySet`, and how many queries it holds. three creates a
    /// fresh set every `beginRender()` and destroys the last one; by then
    /// the last one has been resolved and submitted, so reusing it while
    /// the count fits is the same thing without the churn.
    query_set: Option<(wgpu::QuerySet, u32)>,
    /// `occlusionQueryBuffer` — the last `finishRender()`'s copy, not yet
    /// mapped.
    buffer: Option<Readback>,
    /// `mapAsync()`s in flight, oldest first.
    pending: Vec<Pending>,
    /// `renderContextData.occluded` — `undefined` until a map resolves.
    pub(super) occluded: Option<HashSet<u32>>,
}

/// The query runs over a pass's draws, as `WebGPUBackend.draw()` walks them:
/// a new query begins wherever the object changes to one with
/// `occlusionTest`. Returns the object of each query, in index order.
///
/// three sizes the set by `RenderList.occlusionQueryCount`, which counts the
/// runs in *push* order; the draws run in sorted order, which can have more
/// runs than that. Counting in draw order here means the index can never
/// overrun the set.
pub(super) fn query_objects(draws: impl Iterator<Item = (Option<u32>, bool)>) -> Vec<u32> {
    let mut objects = Vec::new();
    let mut last: Option<Option<u32>> = None;
    for (object, test) in draws {
        if last != Some(object) {
            if test {
                if let Some(id) = object {
                    objects.push(id);
                }
            }
            last = Some(object);
        }
    }
    objects
}

/// The per-renderer occlusion state: every render context's data, and the
/// resolve buffers `occludedResolveCache` shares between them by size.
#[derive(Default)]
pub(super) struct Occlusion {
    contexts: HashMap<ContextKey, OcclusionContext>,
    resolve_buffers: HashMap<u64, wgpu::Buffer>,
}

impl Occlusion {
    /// `renderContextData.occluded` for `key`.
    pub(super) fn occluded(&self, key: ContextKey) -> Option<&HashSet<u32>> {
        self.contexts.get(&key)?.occluded.as_ref()
    }

    /// Whether any `mapAsync()` is outstanding — the one case in which the
    /// renderer has to poll the device before a frame.
    pub(super) fn has_pending(&self) -> bool {
        self.contexts.values().any(|c| !c.pending.is_empty())
    }

    /// The continuation of `resolveOccludedAsync()`: every read-back whose map
    /// has landed becomes its context's `occluded` set — the objects whose
    /// query counted zero samples. Called after a non-blocking device poll.
    pub(super) fn collect(&mut self) {
        for context in self.contexts.values_mut() {
            while let Some(first) = context.pending.first() {
                match first.state.load(Ordering::Acquire) {
                    WAITING => break,
                    MAPPED => {
                        let Pending { readback, .. } = context.pending.remove(0);
                        // A view that cannot be had is a map that failed:
                        // the context keeps the answer it had.
                        let occluded =
                            readback
                                .buffer
                                .slice(..)
                                .get_mapped_range()
                                .ok()
                                .map(|range| {
                                    let results: Vec<u64> = bytemuck::pod_collect_to_vec(&range);
                                    readback
                                        .objects
                                        .iter()
                                        .zip(results)
                                        .filter(|(_, samples)| *samples == 0)
                                        .map(|(&id, _)| id)
                                        .collect()
                                });
                        readback.buffer.unmap();
                        if occluded.is_some() {
                            context.occluded = occluded;
                        }
                    }
                    _ => {
                        context.pending.remove(0);
                    }
                }
            }
        }
    }

    /// `WebGPUBackend.beginRender()`'s occlusion branch. Returns the query set
    /// the pass records into, or `None` when the pass has no occlusion test —
    /// which, as in three, drops a context's stale set.
    pub(super) fn begin(
        &mut self,
        device: &wgpu::Device,
        key: ContextKey,
        count: u32,
    ) -> Option<wgpu::QuerySet> {
        let context = self.contexts.entry(key).or_default();
        if count == 0 {
            context.query_set = None;
            return None;
        }
        match &context.query_set {
            Some((set, capacity)) if *capacity >= count => Some(set.clone()),
            _ => {
                let set = device.create_query_set(&wgpu::QuerySetDescriptor {
                    label: Some("occlusionQuerySet"),
                    ty: wgpu::QueryType::Occlusion,
                    count,
                });
                context.query_set = Some((set.clone(), count));
                Some(set)
            }
        }
    }

    /// `WebGPUBackend.finishRender()`'s occlusion branch, recorded into the
    /// pass's encoder before it is submitted: resolve the set, copy it into a
    /// fresh mappable buffer, and then `resolveOccludedAsync()` — which maps
    /// the buffer the *previous* `finishRender()` left, not this one.
    pub(super) fn finish(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        key: ContextKey,
        query_set: &wgpu::QuerySet,
        objects: Vec<u32>,
    ) {
        let count = objects.len() as u32;
        // 8 byte entries for query results.
        let size = count as u64 * 8;
        let resolve = self
            .resolve_buffers
            .entry(size)
            .or_insert_with(|| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("occlusionQueryResolve"),
                    size,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                })
            })
            .clone();
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("occlusionQueryRead"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.resolve_query_set(query_set, 0..count, &resolve, 0);
        encoder.copy_buffer_to_buffer(&resolve, 0, &read, 0, size);

        let context = self.contexts.entry(key).or_default();
        // `beginRender()`: `currentOcclusionQueryBuffer = occlusionQueryBuffer`.
        let current = context.buffer.replace(Readback {
            buffer: read,
            objects,
        });

        // `resolveOccludedAsync()`.
        if let Some(readback) = current {
            let state = Arc::new(AtomicU8::new(WAITING));
            let signal = state.clone();
            readback
                .buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    signal.store(
                        if result.is_ok() { MAPPED } else { FAILED },
                        Ordering::Release,
                    );
                });
            context.pending.push(Pending { readback, state });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::query_objects;

    #[test]
    fn one_query_per_run_of_a_tested_object() {
        let draws = [
            (None, false),
            (Some(1), false),
            (Some(2), true),
            (Some(2), true),
            (Some(3), false),
            (Some(2), true),
        ];
        assert_eq!(query_objects(draws.into_iter()), vec![2, 2]);
    }

    #[test]
    fn no_tested_object_no_query() {
        let draws = [(Some(1), false), (Some(2), false)];
        assert!(query_objects(draws.into_iter()).is_empty());
    }
}
