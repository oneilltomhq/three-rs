//! Port of the renderer half of `three.js/src/renderers/common/RendererUtils.js`:
//! `saveRendererState()`, `resetRendererState()` and `restoreRendererState()`.
//!
//! A nested render — a pass, a blur chain, a mirror, a PMREM or cube capture
//! drawn from inside another draw's `updateBefore()` — changes the renderer's
//! per-render settings (target, MRT, clear colour, `autoClear`, …) and has to
//! leave them as it found them. Three does that in two ways: the effects call
//! `resetRendererState()` / `restoreRendererState()`, and `PassNode` saves and
//! restores its own longer list by hand. The port had one hand-rolled copy per
//! site, each saving a different subset (#252). Here there is one value,
//! [`RendererState`], holding the union of both lists, and one way to use it:
//! a [`RendererStateScope`] that saves the whole value when it is made and
//! writes it back when it is dropped. Drop covers every exit — the end of the
//! block, an early `return`, a `?`, and unwinding from a panic — so a site
//! cannot forget a field or an exit. Restoring a field nothing changed writes
//! back the value already there, so taking the whole value costs a site
//! nothing it did not ask for.
//!
//! [`Renderer::save_state`] is `saveRendererState()` and
//! [`Renderer::reset_state`] is `resetRendererState()`; both restore on drop,
//! which is `restoreRendererState()`. Between the two the site sets its own
//! overrides through the scope, which derefs to the renderer.
//!
//! What the value holds and what it leaves out:
//!
//! * Three's `saveRendererState()` list, as far as the port has the field:
//!   `toneMapping`, `toneMappingExposure`, `outputColorSpace`, the render
//!   target and its active mipmap level, the render-object function, the
//!   pixel ratio, the MRT, the clear colour and alpha, `autoClear` and the
//!   scissor test. The port has no active cube face: a cube capture draws
//!   each face into a 2-D face target and copies it into its layer.
//! * `PassNode.updateBefore()`'s extra fields: `autoClearColor`,
//!   `autoClearDepth`, `opaque`, `transparent`, `lighting` (the port's
//!   [`Renderer::lighting_enabled`] flag), the camera's layer mask, and the
//!   renderer's context node (the port's `getShadow` and `getAO` hooks).
//!   The port has no `autoClearStencil`.
//! * Not the scene's state (`saveSceneState()`): the port's scene background
//!   and override material belong to the scene, and the sites that clear
//!   them do it on the scene.
//! * Not `RenderContext.fullscreenPass`, which belongs to one render call
//!   rather than to the renderer: [`Renderer::with_fullscreen_pass`] scopes
//!   it around the call that sets it.
//! * Not `ReflectorNode.js`' module-level `_inReflector`. It is a recursion
//!   guard that the reflector sets on entry and clears on exit, not a
//!   setting to put back: three clears it to `false` even when a bouncing
//!   reflector was entered with it already set, so restoring it would differ.
//! * Not the viewport or scissor rectangles, which three does not save
//!   either, nor `RenderPipeline`'s neutral output, which
//!   `Renderer::with_neutral_output` scopes.

use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use crate::core::Layers;
use crate::materials::{MeshBasicNodeMaterial, ToneMapping};
use crate::math::ColorSpace;
use crate::nodes::display::OitRenderObjects;
use crate::nodes::{MrtNode, NodeRef};

use super::pass::ShadowContext;
use super::render_target::RenderTarget;
use super::{OutlineSelection, RenderObjectFunction, Renderer};

/// `renderer.getRenderObjectFunction()`: three's one callback, which the port
/// keeps as one field per effect that installs it. `Default` is three's
/// `setRenderObjectFunction( null )`.
#[derive(Clone, Default)]
pub(crate) struct RenderObjects {
    /// `ToonOutlinePassNode`'s function; see [`Renderer::toon_outline`].
    toon_outline: Option<Rc<MeshBasicNodeMaterial>>,
    /// `RetroPassNode`'s function; see [`Renderer::render_object_function`].
    function: Option<Rc<dyn RenderObjectFunction>>,
    /// `OITPassNode`'s two functions; see [`Renderer::oit`].
    oit: Option<OitRenderObjects>,
    /// `OutlineNode`'s function; see [`Renderer::outline_selection`].
    outline_selection: Option<OutlineSelection>,
}

/// The object `saveRendererState()` fills: every per-render setting of a
/// [`Renderer`] that a nested render may change. See the module docs for what
/// it holds and why.
#[derive(Clone)]
pub(crate) struct RendererState {
    render_target: Option<RenderTarget>,
    active_mipmap_level: u32,
    mrt: Option<MrtNode>,
    clear_color: [f64; 4],
    auto_clear: bool,
    auto_clear_color: bool,
    auto_clear_depth: bool,
    opaque: bool,
    transparent: bool,
    lighting_enabled: bool,
    camera_layers: Option<Layers>,
    context_shadow: Option<ShadowContext>,
    context_ao: Option<NodeRef>,
    render_objects: RenderObjects,
    tone_mapping: ToneMapping,
    tone_mapping_exposure: f64,
    output_color_space: ColorSpace,
    pixel_ratio: f64,
    scissor_test: bool,
}

impl RendererState {
    /// The four lines `resetRendererState()` adds to the save:
    /// `setMRT( null )`, `setRenderObjectFunction( null )`,
    /// `setClearColor( 0x000000, 1 )` and `autoClear = true`.
    fn reset(&mut self) {
        self.mrt = None;
        self.render_objects = RenderObjects::default();
        self.clear_color = [0.0, 0.0, 0.0, 1.0];
        self.auto_clear = true;
    }
}

/// Something a [`RendererState`] can be read from and written back to: the
/// [`Renderer`], and in the unit tests a plain holder, so the scope's
/// save/reset/restore logic is tested without a GPU.
pub(crate) trait HoldsRendererState {
    /// `saveRendererState( renderer )`.
    fn renderer_state(&self) -> RendererState;
    /// `restoreRendererState( renderer, state )`.
    fn set_renderer_state(&mut self, state: RendererState);
}

impl HoldsRendererState for Renderer {
    fn renderer_state(&self) -> RendererState {
        RendererState {
            render_target: self.render_target.clone(),
            active_mipmap_level: self.active_mipmap_level,
            mrt: self.mrt.clone(),
            clear_color: self.clear_color,
            auto_clear: self.auto_clear,
            auto_clear_color: self.auto_clear_color,
            auto_clear_depth: self.auto_clear_depth,
            opaque: self.opaque,
            transparent: self.transparent,
            lighting_enabled: self.lighting_enabled,
            camera_layers: self.camera_layers,
            context_shadow: self.context_shadow.clone(),
            context_ao: self.context_ao.clone(),
            render_objects: RenderObjects {
                toon_outline: self.toon_outline.clone(),
                function: self.render_object_function.clone(),
                oit: self.oit,
                outline_selection: self.outline_selection.clone(),
            },
            tone_mapping: self.tone_mapping,
            tone_mapping_exposure: self.tone_mapping_exposure,
            output_color_space: self.output_color_space,
            pixel_ratio: self.pixel_ratio,
            scissor_test: self.scissor_test,
        }
    }

    fn set_renderer_state(&mut self, state: RendererState) {
        // `setRenderTarget( state.renderTarget, state.activeCubeFace,
        // state.activeMipmapLevel )`.
        self.set_render_target_level(state.render_target, state.active_mipmap_level);
        self.mrt = state.mrt;
        self.clear_color = state.clear_color;
        self.auto_clear = state.auto_clear;
        self.auto_clear_color = state.auto_clear_color;
        self.auto_clear_depth = state.auto_clear_depth;
        self.opaque = state.opaque;
        self.transparent = state.transparent;
        self.lighting_enabled = state.lighting_enabled;
        self.camera_layers = state.camera_layers;
        self.context_shadow = state.context_shadow;
        self.context_ao = state.context_ao;
        self.toon_outline = state.render_objects.toon_outline;
        self.render_object_function = state.render_objects.function;
        self.oit = state.render_objects.oit;
        self.outline_selection = state.render_objects.outline_selection;
        self.tone_mapping = state.tone_mapping;
        self.tone_mapping_exposure = state.tone_mapping_exposure;
        self.output_color_space = state.output_color_space;
        // `setPixelRatio()` also drops the canvas target, so it is only
        // called when the ratio really changed inside the scope.
        if self.pixel_ratio != state.pixel_ratio {
            self.set_pixel_ratio(state.pixel_ratio);
        }
        self.scissor_test = state.scissor_test;
    }
}

/// A saved [`RendererState`], written back to its holder when the scope is
/// dropped: `restoreRendererState()` on every exit, unwinding included.
///
/// The scope derefs to the holder, so a nested render sets its overrides and
/// renders through it exactly as it would through the renderer.
pub(crate) struct RendererStateScope<'a, T: HoldsRendererState = Renderer> {
    holder: &'a mut T,
    /// `Some` until `drop` takes it.
    saved: Option<RendererState>,
}

impl<'a, T: HoldsRendererState> RendererStateScope<'a, T> {
    /// `saveRendererState( holder )`, restored on drop.
    pub(crate) fn save(holder: &'a mut T) -> Self {
        let saved = holder.renderer_state();
        Self {
            holder,
            saved: Some(saved),
        }
    }

    /// `resetRendererState( holder )`, restored on drop: save, then no MRT,
    /// no render-object function, an opaque black clear colour and
    /// `autoClear` on.
    pub(crate) fn reset(holder: &'a mut T) -> Self {
        let scope = Self::save(holder);
        let mut reset = scope
            .saved
            .clone()
            .expect("three-rs: a renderer state scope holds its state until drop");
        reset.reset();
        scope.holder.set_renderer_state(reset);
        scope
    }
}

impl<T: HoldsRendererState> Deref for RendererStateScope<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.holder
    }
}

impl<T: HoldsRendererState> DerefMut for RendererStateScope<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.holder
    }
}

impl<T: HoldsRendererState> Drop for RendererStateScope<'_, T> {
    fn drop(&mut self) {
        if let Some(saved) = self.saved.take() {
            self.holder.set_renderer_state(saved);
        }
    }
}

impl Renderer {
    /// `RendererUtils.saveRendererState( renderer )` for a nested render; the
    /// returned scope restores everything when it is dropped. See
    /// [`RendererState`].
    pub(crate) fn save_state(&mut self) -> RendererStateScope<'_> {
        RendererStateScope::save(self)
    }

    /// `RendererUtils.resetRendererState( renderer )` for a nested render;
    /// the returned scope restores everything when it is dropped. See
    /// [`RendererState`].
    pub(crate) fn reset_state(&mut self) -> RendererStateScope<'_> {
        RendererStateScope::reset(self)
    }

    /// `renderContext.fullscreenPass = scene.isQuadMesh === true` for the
    /// length of one render call, and the outer call's value back after it:
    /// a scene render nested in a quad's draw is not a fullscreen pass, and
    /// the quad's own draw still is once the nested render returns.
    pub(crate) fn with_fullscreen_pass<R>(
        &mut self,
        fullscreen_pass: bool,
        render: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let previous = std::mem::replace(&mut self.fullscreen_pass, fullscreen_pass);
        let result = render(self);
        self.fullscreen_pass = previous;
        result
    }
}

#[cfg(test)]
mod tests {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    use super::*;

    /// A [`HoldsRendererState`] with no GPU behind it.
    struct Holder(RendererState);

    impl HoldsRendererState for Holder {
        fn renderer_state(&self) -> RendererState {
            self.0.clone()
        }

        fn set_renderer_state(&mut self, state: RendererState) {
            self.0 = state;
        }
    }

    /// `Renderer::new()`'s values.
    fn initial() -> RendererState {
        RendererState {
            render_target: None,
            active_mipmap_level: 0,
            mrt: None,
            clear_color: [0.0, 0.0, 0.0, 0.0],
            auto_clear: true,
            auto_clear_color: true,
            auto_clear_depth: true,
            opaque: true,
            transparent: true,
            lighting_enabled: true,
            camera_layers: None,
            context_shadow: None,
            context_ao: None,
            render_objects: RenderObjects::default(),
            tone_mapping: ToneMapping::None,
            tone_mapping_exposure: 1.0,
            output_color_space: ColorSpace::Srgb,
            pixel_ratio: 1.0,
            scissor_test: false,
        }
    }

    /// An outer render's state with every field moved off its default, as a
    /// nested render inside a pass with MRT would find it. Each call makes a
    /// new target, so a test compares against one value it clones.
    fn outer() -> RendererState {
        RendererState {
            render_target: Some(RenderTarget::new(4, 4)),
            active_mipmap_level: 2,
            mrt: Some(MrtNode::default()),
            clear_color: [0.25, 0.5, 0.75, 0.5],
            auto_clear: false,
            auto_clear_color: false,
            auto_clear_depth: false,
            opaque: false,
            transparent: false,
            lighting_enabled: false,
            camera_layers: Some(Layers::default()),
            context_shadow: None,
            context_ao: Some(crate::nodes::tsl::float(1.0)),
            render_objects: RenderObjects {
                oit: Some(OitRenderObjects::Accumulate),
                ..RenderObjects::default()
            },
            tone_mapping: ToneMapping::AcesFilmic,
            tone_mapping_exposure: 2.0,
            output_color_space: ColorSpace::LinearSrgb,
            pixel_ratio: 2.0,
            scissor_test: true,
        }
    }

    /// Field-by-field equality, by identity for the handles.
    fn assert_same(a: &RendererState, b: &RendererState) {
        let target = |s: &RendererState| s.render_target.as_ref().map(|t| t.texture().id());
        assert_eq!(target(a), target(b), "render target");
        assert_eq!(a.active_mipmap_level, b.active_mipmap_level);
        assert_eq!(a.mrt.is_some(), b.mrt.is_some(), "mrt");
        assert_eq!(a.clear_color, b.clear_color);
        assert_eq!(a.auto_clear, b.auto_clear);
        assert_eq!(a.auto_clear_color, b.auto_clear_color);
        assert_eq!(a.auto_clear_depth, b.auto_clear_depth);
        assert_eq!(a.opaque, b.opaque);
        assert_eq!(a.transparent, b.transparent);
        assert_eq!(a.lighting_enabled, b.lighting_enabled);
        assert_eq!(a.camera_layers, b.camera_layers);
        assert_eq!(a.context_shadow.is_some(), b.context_shadow.is_some());
        assert_eq!(
            a.context_ao.as_ref().map(NodeRef::key),
            b.context_ao.as_ref().map(NodeRef::key),
            "context ao"
        );
        assert_eq!(a.render_objects.oit, b.render_objects.oit);
        assert_eq!(
            a.render_objects.toon_outline.is_some(),
            b.render_objects.toon_outline.is_some()
        );
        assert_eq!(
            a.render_objects.function.is_some(),
            b.render_objects.function.is_some()
        );
        assert_eq!(
            a.render_objects.outline_selection.is_some(),
            b.render_objects.outline_selection.is_some()
        );
        assert_eq!(a.tone_mapping, b.tone_mapping);
        assert_eq!(a.tone_mapping_exposure, b.tone_mapping_exposure);
        assert_eq!(a.output_color_space, b.output_color_space);
        assert_eq!(a.pixel_ratio, b.pixel_ratio);
        assert_eq!(a.scissor_test, b.scissor_test);
    }

    /// Every field the scope's body changes is back when it ends.
    #[test]
    fn save_restores_every_field_on_drop() {
        let start = outer();
        let mut holder = Holder(start.clone());
        {
            let mut scope = RendererStateScope::save(&mut holder);
            assert_same(&scope.0, &start);
            scope.0 = initial();
            scope.0.render_target = Some(RenderTarget::new(2, 2));
        }
        assert_same(&holder.0, &start);
    }

    /// `resetRendererState()` changes exactly its four fields and keeps the
    /// rest of the outer state, the render target included; the drop puts
    /// the four back.
    #[test]
    fn reset_changes_its_four_fields_and_restores_them() {
        let start = outer();
        let mut holder = Holder(start.clone());
        {
            let scope = RendererStateScope::reset(&mut holder);
            let mut expected = start.clone();
            expected.mrt = None;
            expected.render_objects = RenderObjects::default();
            expected.clear_color = [0.0, 0.0, 0.0, 1.0];
            expected.auto_clear = true;
            assert_same(&scope.0, &expected);
        }
        assert_same(&holder.0, &start);
    }

    /// Scopes nest: an inner scope restores to the outer one's overrides,
    /// and the outer one to the original.
    #[test]
    fn nested_scopes_restore_in_order() {
        let mut holder = Holder(initial());
        {
            let mut outer_scope = RendererStateScope::save(&mut holder);
            outer_scope.0.auto_clear_depth = false;
            {
                let mut inner = RendererStateScope::reset(&mut *outer_scope);
                inner.0.auto_clear_depth = true;
                inner.0.clear_color = [1.0, 1.0, 1.0, 1.0];
            }
            assert!(!outer_scope.0.auto_clear_depth);
            assert_eq!(outer_scope.0.clear_color, [0.0, 0.0, 0.0, 0.0]);
        }
        assert_same(&holder.0, &initial());
    }

    /// An early return out of the scope's block restores too.
    #[test]
    fn early_return_restores() {
        fn nested(holder: &mut Holder, bail: bool) -> Option<()> {
            let mut scope = RendererStateScope::reset(holder);
            scope.0.opaque = false;
            if bail {
                return None;
            }
            scope.0.transparent = false;
            Some(())
        }
        let mut holder = Holder(initial());
        assert_eq!(nested(&mut holder, true), None);
        assert_same(&holder.0, &initial());
        assert_eq!(nested(&mut holder, false), Some(()));
        assert_same(&holder.0, &initial());
    }

    /// A panic inside the nested render unwinds through the scope, which
    /// still restores the outer state.
    #[test]
    fn panic_restores() {
        let start = outer();
        let mut holder = Holder(start.clone());
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut scope = RendererStateScope::reset(&mut holder);
            scope.0.render_target = None;
            panic!("nested render failed");
        }));
        assert!(result.is_err());
        assert_same(&holder.0, &start);
    }
}
