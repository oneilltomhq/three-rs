//! Port of `three.js/examples/jsm/tsl/display/BleachBypass.js`.

use crate::nodes::tsl::{block, float, luminance, mix, to_var, vec4_join};
use crate::nodes::NodeRef;

/// `bleach( color, opacity = 1 )` — the bleach-bypass film look: an overlay
/// of `color` with its own luminance, mixed in by `color.a · opacity`.
///
/// `L = min( 1, max( 0, 10 · ( lum - 0.45 ) ) )` picks between the multiply
/// half (`2 · lum · rgb`) and the screen half (`1 - 2 · ( 1 - lum ) · ( 1 -
/// rgb )`) of the overlay; the result is `A2 · overlay + rgb · ( 1 - A2 )`
/// with `A2 = color.a · opacity`, and `color.a` is kept.
///
/// A `Fn()` with no layout, so inlined. Its last step is an `addAssign`,
/// which makes the mixed colour a `var` in the generated code.
pub fn bleach(color: impl Into<NodeRef>, opacity: impl Into<NodeRef>) -> NodeRef {
    let (base, opacity) = (color.into(), opacity.into());

    let lum = luminance(base.rgb());
    let blend = lum.clone().to(crate::nodes::Type::Vec3);

    let l = float(1.0).min(float(0.0).max(float(10.0).mul(lum.sub(0.45))));

    let result1 = blend.clone().mul(base.rgb()).mul(2.0);
    let result2 = float(2.0)
        .mul(blend.one_minus())
        .mul(base.rgb().one_minus())
        .one_minus();

    let new_color = mix(result1, result2, l);

    let a2 = base.a().mul(opacity);

    let mix_rgb = to_var(None, a2.clone().mul(new_color.rgb()));

    let add = mix_rgb.add_assign(base.rgb().mul(a2.one_minus()));

    block(
        vec![mix_rgb.clone(), add],
        vec4_join(vec![mix_rgb, base.a()]),
    )
}
