//! Port of `three.js/src/animation/AnimationUtils.js`.
//!
//! Only the clip-independent helpers live here. The three.js module also
//! exports `subclip()` and `makeClipAdditive()`, which operate on
//! `AnimationClip` / `KeyframeTrack`; they are **deferred to the clip port**
//! and will land next to those types rather than here.
//!
//! Three's `convertArray( array, type )` and `isTypedArray( object )` have no
//! Rust counterpart and are deliberately not exposed:
//!
//! * `convertArray` exists so a keyframe track can re-wrap a plain `Array` in
//!   whatever typed array the track stores (`Float32Array`, `Int32Array`, …).
//!   Every buffer in this port is already an f64 `Vec<f64>`, so the conversion
//!   is the identity and the call sites simply keep their `Vec`.
//! * `isTypedArray` distinguishes a typed array from a plain `Array` at
//!   runtime. In Rust the distinction is in the type, so the question cannot
//!   be asked.
//!
//! `hasTangents( settings )` is likewise a JS `undefined` check over a
//! loose keyframe-track settings object; it belongs with the track parsing
//! code that owns that struct.

/// `getKeyframeOrder( times )` — the permutation by which `times` (and, via
/// [`sorted_array`], the matching values) can be sorted.
///
/// Three sorts the index array with `Array.prototype.sort`, which is required
/// to be stable, so equal times keep their original relative order; Rust's
/// [`slice::sort_by`] is stable too.
pub fn get_keyframe_order(times: &[f64]) -> Vec<usize> {
    let n = times.len();
    let mut result: Vec<usize> = (0..n).collect();

    // `compareTime( i, j ) { return times[ i ] - times[ j ]; }`. A NaN time
    // makes the JS comparator return NaN, which `sort` treats as "keep the
    // current order"; `Equal` is the same instruction to a stable sort.
    result.sort_by(|&i, &j| {
        (times[i] - times[j])
            .partial_cmp(&0.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    result
}

/// `sortedArray( values, stride, order )` — reorders `values` in blocks of
/// `stride` according to the `order` from [`get_keyframe_order`].
pub fn sorted_array(values: &[f64], stride: usize, order: &[usize]) -> Vec<f64> {
    let n_values = values.len();
    let mut result = vec![0.0; n_values];

    let mut dst_offset = 0;
    let mut i = 0;
    while dst_offset != n_values {
        let src_offset = order[i] * stride;

        for j in 0..stride {
            result[dst_offset] = values[src_offset + j];
            dst_offset += 1;
        }

        i += 1;
    }

    result
}

/// `flattenJSON( jsonKeys, times, values, valuePropertyName )` — parses the
/// array-of-structs keyframe format that `AnimationClip.parse()` feeds in,
/// appending to `times` and `values`.
///
/// Three picks its branch from the first key that actually carries
/// `valuePropertyName`: an `Array` value is spread into `values`, a
/// `THREE.Math`-ish object is written through its `toArray()`, and anything
/// else is pushed as-is. JSON has no methods, so the `toArray` branch is
/// unreachable over [`serde_json::Value`] and is not ported.
///
/// A key missing `time` pushes `undefined` in Three; here it pushes `NaN`,
/// which is what that `undefined` becomes the moment the times are read as
/// numbers.
pub fn flatten_json(
    json_keys: &[serde_json::Value],
    times: &mut Vec<f64>,
    values: &mut Vec<f64>,
    value_property_name: &str,
) {
    let number = |v: &serde_json::Value| v.as_f64().unwrap_or(f64::NAN);

    // `let i = 1, key = jsonKeys[ 0 ];` then skip keys without the property.
    let mut i = 1;
    let mut key = json_keys.first();

    while let Some(k) = key {
        if k.get(value_property_name).is_some() {
            break;
        }
        key = json_keys.get(i);
        i += 1;
    }

    let Some(first) = key else {
        return; // no data
    };
    let value = &first[value_property_name];

    if value.is_array() {
        while let Some(k) = key {
            if let Some(value) = k.get(value_property_name) {
                times.push(number(&k["time"]));
                // `values.push( ...value )`
                values.extend(value.as_array().into_iter().flatten().map(number));
            }

            key = json_keys.get(i);
            i += 1;
        }
    } else {
        // Otherwise push as-is.
        while let Some(k) = key {
            if let Some(value) = k.get(value_property_name) {
                times.push(number(&k["time"]));
                values.push(number(value));
            }

            key = json_keys.get(i);
            i += 1;
        }
    }
}
