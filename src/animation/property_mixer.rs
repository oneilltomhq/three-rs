//! Port of `three.js/src/animation/PropertyMixer.js`.
//!
//! The one structural change: Three's `binding` is a `PropertyBinding` (or an
//! `AnimationObjectGroup` composite), which this crate does not have yet, so the
//! field is a [`BindingTarget`] — the `getValue` / `setValue` pair is all
//! `PropertyMixer` ever uses of it (see
//! [`crate::animation::binding_target`]). Three's `this.valueSize` is therefore
//! read back off the target with `value_size()` where the JS reads
//! `binding.valueSize`, but it is still cached in a field exactly as the JS
//! caches the constructor argument.
//!
//! Three dispatches the mix functions off the *string* `typeName` of the track
//! (`'quaternion'`, `'string'`/`'bool'`, everything else); here that is the
//! crate's [`TrackValueType`] discriminant, and the three dispatch branches are
//! collapsed into [`MixKind`] — a JS method-reference field (`_mixBufferRegion`)
//! has no direct Rust equivalent, and a function pointer cannot carry the
//! `_workIndex` that `_slerpAdditive` needs.

use crate::animation::binding_target::BindingTarget;
use crate::animation::keyframe_track::TrackValueType;
use crate::math::Quaternion;

/// Which of Three's mix-function triples (`_mixBufferRegion`,
/// `_mixBufferRegionAdditive`, `_setIdentity`) this mixer uses — the
/// `switch ( typeName )` in the JS constructor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MixKind {
    /// `'quaternion'`: `_slerp` / `_slerpAdditive` /
    /// `_setAdditiveIdentityQuaternion`, and the extra `work` buffer region.
    Quaternion,
    /// `'string'` and `'bool'`: `_select` for both mix functions (additive is
    /// not meaningful for non-numeric types) and `_setAdditiveIdentityOther`.
    Select,
    /// Everything else: `_lerp` / `_lerpAdditive` /
    /// `_setAdditiveIdentityNumeric`.
    Lerp,
}

impl MixKind {
    /// The branch Three's `switch ( typeName )` takes for a track type.
    pub fn for_value_type(type_name: TrackValueType) -> Self {
        match type_name {
            TrackValueType::Quaternion => Self::Quaternion,
            TrackValueType::Bool | TrackValueType::String => Self::Select,
            TrackValueType::Color | TrackValueType::Number | TrackValueType::Vector => Self::Lerp,
        }
    }
}

/// Buffered scene graph property that allows weighted accumulation; used
/// internally.
///
/// buffer layout: `[ incoming | accu0 | accu1 | orig | addAccu | (optional
/// work) ]`, each region `value_size` long.
///
/// Interpolators can use `.buffer` as their result; the data then goes to
/// `incoming`. `accu0` and `accu1` are used frame-interleaved for the cumulative
/// result and are compared to detect changes. `orig` stores the original state
/// of the property. `add` is used for additive cumulative results. `work` is
/// only present for quaternion types, to hold intermediate quaternion
/// multiplication results.
pub struct PropertyMixer {
    /// The property binding. Three: `binding` (a `PropertyBinding`).
    pub binding: Box<dyn BindingTarget>,

    /// The keyframe track value size. Three: `valueSize`.
    pub value_size: usize,

    /// The mix buffer. Three: `buffer` (a `Float64Array`, or a plain `Array`
    /// for the string/bool case — there is no separate untyped variant here,
    /// since this crate's tracks are f64-valued throughout).
    pub buffer: Vec<f64>,

    /// Which mix-function triple the `typeName` switch selected. Three:
    /// `_mixBufferRegion` / `_mixBufferRegionAdditive` / `_setIdentity`.
    mix_kind: MixKind,

    /// Three: `_origIndex` (always 3).
    orig_index: usize,
    /// Three: `_addIndex` (always 4).
    add_index: usize,
    /// Three: `_workIndex` (5, quaternion only).
    work_index: usize,

    /// Accumulated weight of the property binding. Three: `cumulativeWeight`.
    pub cumulative_weight: f64,

    /// Accumulated additive weight of the property binding. Three:
    /// `cumulativeWeightAdditive`.
    pub cumulative_weight_additive: f64,

    /// Number of active keyframe tracks currently using this property binding.
    /// Three: `useCount`.
    pub use_count: usize,

    /// Number of keyframe tracks referencing this property binding. Three:
    /// `referenceCount`.
    pub reference_count: usize,
}

impl PropertyMixer {
    /// `new PropertyMixer( binding, typeName, valueSize )`.
    pub fn new(
        binding: Box<dyn BindingTarget>,
        type_name: TrackValueType,
        value_size: usize,
    ) -> Self {
        let mix_kind = MixKind::for_value_type(type_name);

        let regions = match mix_kind {
            MixKind::Quaternion => 6,
            _ => 5,
        };

        Self {
            binding,
            value_size,
            buffer: vec![0.0; value_size * regions],
            mix_kind,
            orig_index: 3,
            add_index: 4,
            work_index: 5,
            cumulative_weight: 0.0,
            cumulative_weight_additive: 0.0,
            use_count: 0,
            reference_count: 0,
        }
    }

    /// Accumulates data in the `incoming` region into `accu<i>`.
    pub fn accumulate(&mut self, accu_index: usize, weight: f64) {
        // note: happily accumulating nothing when weight = 0, the caller knows
        // the weight and shouldn't have made the call in the first place

        let stride = self.value_size;
        let offset = accu_index * stride + stride;

        let mut current_weight = self.cumulative_weight;

        if current_weight == 0.0 {
            // accuN := incoming * weight

            for i in 0..stride {
                self.buffer[offset + i] = self.buffer[i];
            }

            current_weight = weight;
        } else {
            // accuN := accuN + incoming * weight

            current_weight += weight;
            let mix = weight / current_weight;
            self.mix_buffer_region(offset, 0, mix, stride);
        }

        self.cumulative_weight = current_weight;
    }

    /// Accumulates data in the `incoming` region into `add`.
    pub fn accumulate_additive(&mut self, weight: f64) {
        let stride = self.value_size;
        let offset = stride * self.add_index;

        if self.cumulative_weight_additive == 0.0 {
            // add = identity

            self.set_identity();
        }

        // add := add + incoming * weight

        self.mix_buffer_region_additive(offset, 0, weight, stride);
        self.cumulative_weight_additive += weight;
    }

    /// Applies the state of `accu<i>` to the binding when accus differ.
    pub fn apply(&mut self, accu_index: usize) {
        let stride = self.value_size;
        let offset = accu_index * stride + stride;

        let weight = self.cumulative_weight;
        let weight_additive = self.cumulative_weight_additive;

        self.cumulative_weight = 0.0;
        self.cumulative_weight_additive = 0.0;

        if weight < 1.0 {
            // accuN := accuN + original * ( 1 - cumulativeWeight )

            let original_value_offset = stride * self.orig_index;

            self.mix_buffer_region(offset, original_value_offset, 1.0 - weight, stride);
        }

        if weight_additive > 0.0 {
            // accuN := accuN + additive accuN

            self.mix_buffer_region_additive(offset, self.add_index * stride, 1.0, stride);
        }

        for i in stride..(stride + stride) {
            if self.buffer[i] != self.buffer[i + stride] {
                // value has changed -> update scene graph

                self.binding.set_value(&self.buffer, offset);
                break;
            }
        }
    }

    /// Remembers the state of the bound property and copies it to both accus.
    pub fn save_original_state(&mut self) {
        let stride = self.value_size;
        let original_value_offset = stride * self.orig_index;

        self.binding
            .get_value(&mut self.buffer, original_value_offset);

        // accu[0..1] := orig -- initially detect changes against the original
        for i in stride..original_value_offset {
            self.buffer[i] = self.buffer[original_value_offset + (i % stride)];
        }

        // Add to identity for additive
        self.set_identity();

        self.cumulative_weight = 0.0;
        self.cumulative_weight_additive = 0.0;
    }

    /// Applies the state previously taken via [`PropertyMixer::save_original_state`]
    /// to the binding.
    pub fn restore_original_state(&mut self) {
        let original_value_offset = self.value_size * 3;
        self.binding.set_value(&self.buffer, original_value_offset);
    }

    // internals

    /// `_setIdentity` — the branch picked by the `typeName` switch.
    fn set_identity(&mut self) {
        match self.mix_kind {
            MixKind::Quaternion => self.set_additive_identity_quaternion(),
            MixKind::Select => self.set_additive_identity_other(),
            MixKind::Lerp => self.set_additive_identity_numeric(),
        }
    }

    /// `_setAdditiveIdentityNumeric`.
    fn set_additive_identity_numeric(&mut self) {
        let start_index = self.add_index * self.value_size;
        let end_index = start_index + self.value_size;

        for i in start_index..end_index {
            self.buffer[i] = 0.0;
        }
    }

    /// `_setAdditiveIdentityQuaternion`.
    fn set_additive_identity_quaternion(&mut self) {
        self.set_additive_identity_numeric();
        self.buffer[self.add_index * self.value_size + 3] = 1.0;
    }

    /// `_setAdditiveIdentityOther`.
    fn set_additive_identity_other(&mut self) {
        let start_index = self.orig_index * self.value_size;
        let target_index = self.add_index * self.value_size;

        for i in 0..self.value_size {
            self.buffer[target_index + i] = self.buffer[start_index + i];
        }
    }

    // mix functions

    /// `_mixBufferRegion`.
    fn mix_buffer_region(&mut self, dst_offset: usize, src_offset: usize, t: f64, stride: usize) {
        match self.mix_kind {
            MixKind::Quaternion => Self::slerp(&mut self.buffer, dst_offset, src_offset, t),
            MixKind::Select => Self::select(&mut self.buffer, dst_offset, src_offset, t, stride),
            MixKind::Lerp => Self::lerp(&mut self.buffer, dst_offset, src_offset, t, stride),
        }
    }

    /// `_mixBufferRegionAdditive`.
    fn mix_buffer_region_additive(
        &mut self,
        dst_offset: usize,
        src_offset: usize,
        t: f64,
        stride: usize,
    ) {
        match self.mix_kind {
            MixKind::Quaternion => {
                let work_offset = self.work_index * stride;
                Self::slerp_additive(&mut self.buffer, dst_offset, src_offset, t, work_offset);
            }
            MixKind::Select => Self::select(&mut self.buffer, dst_offset, src_offset, t, stride),
            MixKind::Lerp => {
                Self::lerp_additive(&mut self.buffer, dst_offset, src_offset, t, stride);
            }
        }
    }

    /// `_select`.
    fn select(buffer: &mut [f64], dst_offset: usize, src_offset: usize, t: f64, stride: usize) {
        if t >= 0.5 {
            for i in 0..stride {
                buffer[dst_offset + i] = buffer[src_offset + i];
            }
        }
    }

    /// `_slerp`.
    fn slerp(buffer: &mut [f64], dst_offset: usize, src_offset: usize, t: f64) {
        // Three passes `buffer` as dst, src0 and src1 at once;
        // `Quaternion::slerp_flat` reads its sources up front, so the aliasing
        // is resolved here by copying the two operands out first.
        let mut src0 = [0.0; 4];
        let mut src1 = [0.0; 4];
        src0.copy_from_slice(&buffer[dst_offset..dst_offset + 4]);
        src1.copy_from_slice(&buffer[src_offset..src_offset + 4]);

        Quaternion::slerp_flat(buffer, dst_offset, &src0, 0, &src1, 0, t);
    }

    /// `_slerpAdditive`. Three derives `workOffset` from `this._workIndex`;
    /// it is passed in here so the function can stay free of `&self`.
    fn slerp_additive(
        buffer: &mut [f64],
        dst_offset: usize,
        src_offset: usize,
        t: f64,
        work_offset: usize,
    ) {
        let mut src0 = [0.0; 4];
        let mut src1 = [0.0; 4];
        src0.copy_from_slice(&buffer[dst_offset..dst_offset + 4]);
        src1.copy_from_slice(&buffer[src_offset..src_offset + 4]);

        // Store result in intermediate buffer offset
        Quaternion::multiply_quaternions_flat(buffer, work_offset, &src0, 0, &src1, 0);

        // Slerp to the intermediate result
        Self::slerp(buffer, dst_offset, work_offset, t);
    }

    /// `_lerp`.
    fn lerp(buffer: &mut [f64], dst_offset: usize, src_offset: usize, t: f64, stride: usize) {
        let s = 1.0 - t;

        for i in 0..stride {
            let j = dst_offset + i;

            buffer[j] = buffer[j] * s + buffer[src_offset + i] * t;
        }
    }

    /// `_lerpAdditive`.
    fn lerp_additive(
        buffer: &mut [f64],
        dst_offset: usize,
        src_offset: usize,
        t: f64,
        stride: usize,
    ) {
        for i in 0..stride {
            let j = dst_offset + i;

            buffer[j] += buffer[src_offset + i] * t;
        }
    }
}
