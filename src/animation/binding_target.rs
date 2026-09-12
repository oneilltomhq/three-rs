//! The seam where `PropertyBinding`'s getter/setter half will meet the scene
//! graph.
//!
//! Three's `PropertyBinding` does two separable jobs: it parses a track name
//! (`parseTrackName`, ported in [`crate::animation::property_binding`]) and it
//! resolves that name against a live `Object3D` tree into a `getValue` /
//! `setValue` pair, picking one of sixteen `GetterByBindingType` /
//! `SetterByBindingTypeAndVersioning` closures depending on whether the target
//! property is a number, an array, a `Color`/`Vector`/`Quaternion` with
//! `fromArray`, or a morph-target-influence name, and whether it needs
//! `needsUpdate` or `matrixWorldNeedsUpdate` flagged afterwards.
//!
//! The second job needs an `Object3D` tree with named property access, which
//! this crate does not have yet (it is being built on a parallel branch). So
//! the mixer side is written against these two traits instead:
//!
//! - [`BindingTarget`] is one resolved property: read it into a flat buffer,
//!   write it back from one. That is the whole of what `PropertyMixer` and
//!   `AnimationAction` ever ask a binding to do — they only ever see
//!   `getValue( buffer, offset )` and `setValue( buffer, offset )`.
//! - [`TargetResolver`] is the tree: hand it a parsed track name, get a
//!   `BindingTarget` back. `Object3D` implements this once it exists, and
//!   `AnimationObjectGroup` implements it by fanning out over its members.
//!
//! `bind` / `unbind` stay on the trait because Three's `Composite` binding and
//! `PropertyBinding.bind()` are lazy: the first `getValue` triggers the
//! resolve, and `unbind` drops it so a re-bound root is re-resolved.

use crate::animation::property_binding::ParsedTrackName;

/// One resolved animatable property, as a flat run of `valueSize` f64s.
///
/// The offset is into the caller's buffer, not into the target: Three's
/// `getValue( buffer, offset )` / `setValue( buffer, offset )` read and write
/// `buffer[ offset .. offset + valueSize ]`.
pub trait BindingTarget {
    /// `getValue( buffer, offset )`.
    fn get_value(&self, buffer: &mut [f64], offset: usize);

    /// `setValue( buffer, offset )`.
    fn set_value(&mut self, buffer: &[f64], offset: usize);

    /// `valueSize` — how many f64s this property occupies.
    fn value_size(&self) -> usize;

    /// `bind()`. Three resolves lazily and leaves the bound state on the
    /// binding; a target that is already resolved can leave this empty.
    fn bind(&mut self) {}

    /// `unbind()`.
    fn unbind(&mut self) {}
}

/// The scene-graph side: resolve a parsed track name to a [`BindingTarget`].
///
/// `Object3D` (and `AnimationObjectGroup`, over its members) implements this.
/// `None` is Three's `console.error( 'THREE.PropertyBinding: Trying to update
/// node for track: … but it wasn't found.' )` — a track that does not resolve is
/// skipped, not fatal.
pub trait TargetResolver {
    /// `PropertyBinding.bind()`: `findNode` plus the property lookup.
    fn resolve(&mut self, parsed: &ParsedTrackName) -> Option<Box<dyn BindingTarget>>;
}

/// A [`BindingTarget`] over a plain buffer, for tests and for the mixer work
/// that cannot wait for the object tree: it behaves exactly as Three's
/// `Direct` + `EntireArray` binding over a `Float64Array` property.
#[derive(Clone, Debug, Default)]
pub struct BufferTarget {
    /// The property's current value.
    pub value: Vec<f64>,
    /// How many times [`BindingTarget::set_value`] has run, the stand-in for
    /// the `needsUpdate` flag Three's versioned setters raise.
    pub set_count: usize,
}

impl BufferTarget {
    /// A target holding `value`.
    pub fn new(value: Vec<f64>) -> Self {
        Self {
            value,
            set_count: 0,
        }
    }
}

impl BindingTarget for BufferTarget {
    fn get_value(&self, buffer: &mut [f64], offset: usize) {
        for (i, &v) in self.value.iter().enumerate() {
            buffer[offset + i] = v;
        }
    }

    fn set_value(&mut self, buffer: &[f64], offset: usize) {
        let n = self.value.len();
        self.value.copy_from_slice(&buffer[offset..offset + n]);
        self.set_count += 1;
    }

    fn value_size(&self) -> usize {
        self.value.len()
    }
}
