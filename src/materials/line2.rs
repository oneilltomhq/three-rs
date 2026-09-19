//! Port of `three.js/src/materials/nodes/Line2NodeMaterial.js` — the fat-line
//! material: `mvpLine`, `trimSegmentAlpha` and `alphaLine`.
//!
//! It is core, not an addon, because three.js ships it in `src/materials/nodes/`
//! and because it needs a `MaterialKind`, a `setupPosition` seam and six
//! `UniformSource` variants — none of which a downstream crate can reach. The
//! geometry and object half (`LineSegmentsGeometry`, `LineGeometry`,
//! `LineSegments2`, `Line2`) is `examples/jsm/lines/` and lives in
//! [`crate::addons::lines`].
//!
//! # What is ported, and what is not
//!
//! Three's material branches at setup time on three flags. Ported here:
//!
//! * `_useWorldUnits = false` — the **screen-space** branch of `mvpLine` and
//!   the round-endcap branch of `alphaLine`.
//! * `_useAlphaToCoverage` — both branches, keyed on
//!   [`Material::alpha_to_coverage`](crate::materials::MeshBasicNodeMaterial::alpha_to_coverage).
//!
//! Not ported: `_useDash` (the `instanceDistance*` attributes, `lineDistance`,
//! `dashSize` / `gapSize` and the `mod`-discard) and `_useWorldUnits`
//! (`closestLineToLine` and the world-space ribbon). Both are off the graded
//! frame's path — `webgpu_lines_fat` sets `dashed: false` and leaves world
//! units at their default — and both want node shapes the port does not have
//! yet (a `varyingProperty` assigned in the vertex stage before it is read).
//! `docs/webgpu_lines_fat-progress.md` records them as the rung's gaps.

use std::rc::Rc;

use crate::nodes::lines::LineSegmentsAttributes;
use crate::nodes::node::FnDef;
use crate::nodes::node::Type;
use crate::nodes::tsl::*;
use crate::nodes::NodeRef;

/// `trimSegmentAlpha( { start, end } )` — a real WGSL `fn` with a layout,
/// called twice, exactly as three's `Fn( …, { start: 'vec4', end: 'vec4',
/// return: 'float' } )` is.
///
/// "we need different nearEstimate formula for reversed and default depth
/// buffer — `a` is positive with a reversed depth buffer so it can be used for
/// controlling the code flow".
fn trim_segment_alpha() -> Rc<FnDef> {
    shader_fn(
        None,
        vec![("start", Type::Vec4), ("end", Type::Vec4)],
        Type::F32,
        |args| {
            let (start, end) = (args[0].clone(), args[1].clone());
            // 3rd entry in the 3rd column, and the 3rd entry in the 4th.
            let a = camera_projection_matrix().element(2).element(2);
            let b = camera_projection_matrix().element(3).element(2);
            let near_estimate = a.greater_than(float(0.0)).select(
                b.negate().div(a.add(float(1.0))),
                b.mul(float(-0.5)).div(a.clone()),
            );
            near_estimate.sub(start.z()).div(end.z().sub(start.z()))
        },
    )
}

/// `mvpLine` — the screen-space branch: each segment becomes a quad in NDC,
/// widened by `materialLineWidth` screen pixels and extended by half a width at
/// each end for the round caps.
///
/// Returns the statements to flow into the vertex stage and the clip-space
/// position they leave in the `clip` var.
fn mvp_line(attributes: &LineSegmentsAttributes) -> (Vec<NodeRef>, NodeRef) {
    let (instance_start, instance_end) = attributes.start_end();

    // camera space
    let start = to_var(
        Some("start"),
        model_view_matrix().mul(vec4_join(vec![instance_start, float(1.0)])),
    );
    let end = to_var(
        Some("end"),
        model_view_matrix().mul(vec4_join(vec![instance_end, float(1.0)])),
    );

    let mut statements = vec![start.clone(), end.clone()];

    // "special case for perspective projection, and segments that terminate
    // either in, or behind, the camera plane" — 4th entry in the 3rd column.
    let perspective = camera_projection_matrix()
        .element(2)
        .element(3)
        .equal(float(-1.0));
    let trim = trim_segment_alpha();
    statements.push(if_then(
        perspective,
        vec![if_else_if(
            start
                .z()
                .less_than(float(0.0))
                .and(end.z().greater_than(float(0.0))),
            vec![end.assign(vec4_join(vec![
                mix(
                    start.xyz(),
                    end.xyz(),
                    call(&trim, vec![start.clone(), end.clone()]),
                ),
                end.w(),
            ]))],
            end.z()
                .less_than(float(0.0))
                .and(start.z().greater_than_equal(float(0.0))),
            vec![start.assign(vec4_join(vec![
                mix(
                    end.xyz(),
                    start.xyz(),
                    call(&trim, vec![end.clone(), start.clone()]),
                ),
                start.w(),
            ]))],
        )],
    ));

    // clip space. Neither is `toVar()`'d in three; both are read twice, so the
    // builder hoists them — `clipEnd` first, because `dir` reads `ndcEnd`
    // first, which is why three's dump numbers it `nodeVar0`.
    let clip_start = to_var(None, camera_projection_matrix().mul(start.clone()));
    let clip_end = to_var(None, camera_projection_matrix().mul(end.clone()));

    // ndc space, and the direction between the two ends
    let ndc_start = clip_start.xyz().div(clip_start.w());
    let ndc_end = clip_end.xyz().div(clip_end.w());
    let dir = to_var(None, ndc_end.xy().sub(ndc_start.xy()));

    // account for clip-space aspect ratio. `viewport` is the *pass'* rectangle,
    // so a 125-high inset makes the same line four times wider than a 500-high
    // frame does.
    let aspect = to_var(None, viewport().z().div(viewport().w()));

    statements.push(dir.x().assign(dir.x().mul(aspect.clone())));
    statements.push(dir.assign(dir.normalize()));

    let clip = to_var(None, vec4(0.0, 0.0, 0.0, 1.0));
    statements.push(clip.clone());

    let offset = to_var(Some("offset"), vec2_join(vec![dir.y(), dir.x().negate()]));
    statements.push(offset.clone());

    // undo aspect ratio adjustment
    statements.push(dir.x().assign(dir.x().div(aspect.clone())));
    statements.push(offset.x().assign(offset.x().div(aspect)));

    // sign flip
    statements.push(
        offset.assign(
            position_geometry()
                .x()
                .less_than(float(0.0))
                .select(offset.negate(), offset.clone()),
        ),
    );

    // endcaps
    statements.push(if_else_if(
        position_geometry().y().less_than(float(0.0)),
        vec![offset.assign(offset.sub(dir.clone()))],
        position_geometry().y().greater_than(float(1.0)),
        vec![offset.assign(offset.add(dir.clone()))],
    ));

    // adjust for linewidth, then for the clip-space to screen-space conversion
    statements.push(offset.assign(offset.mul(material_line_width())));
    statements.push(offset.assign(offset.div(viewport().w().div(screen_dpr()))));

    // select end
    statements.push(
        clip.assign(
            position_geometry()
                .y()
                .less_than(float(0.5))
                .select(clip_start, clip_end),
        ),
    );

    // back to clip space
    statements.push(offset.assign(offset.mul(clip.w())));
    statements.push(clip.assign(clip.add(vec4_join(vec![offset, float(0.0), float(0.0)]))));

    (statements, clip)
}

/// `Line2NodeMaterial.setupPosition()` — the whole fat-line transform, pushed
/// **back** into local space so that the ordinary MVP tail can re-do it:
///
/// ```ignore
/// const localPosition = modelWorldMatrixInverse
///     .mul( cameraWorldMatrix ).mul( cameraProjectionMatrixInverse ).mul( mvpLine );
/// positionLocal.assign( localPosition.xyz.div( localPosition.w ) );
/// ```
///
/// The round trip is **not** algebraically the identity in `f32`, and it is the
/// only thing that lets the fat line reuse `modelViewProjection`. Do not
/// simplify it: `v_positionView` and `v_modelViewProjection` still appear after
/// it in three's dump, and so do they here.
pub fn setup_position(attributes: &LineSegmentsAttributes) -> NodeRef {
    let (statements, clip) = mvp_line(attributes);
    let local = to_var(
        None,
        model_world_matrix_inverse()
            .mul(camera_world_matrix())
            .mul(camera_projection_matrix_inverse())
            .mul(clip),
    );
    position_local().assign(block(statements, local.xyz().div(local.w())))
}

/// `alphaLine` — the round-endcap coverage of a screen-space fat line.
///
/// The quad runs from `uv.y = -2` to `2` with the segment itself between `-1`
/// and `1`, so `abs( uv.y ) > 1` is inside one of the two caps and the fragment
/// is kept only if it is inside the cap's unit circle.
///
/// With `alphaToCoverage` the circle is antialiased by `fwidth` instead of cut
/// by a `discard`; three also requires `renderer.currentSamples > 0` for that
/// branch, which the port folds into the material flag because a material with
/// `alphaToCoverage` on an unsampled target is not a case any example makes.
fn alpha_line(alpha_to_coverage: bool) -> NodeRef {
    let v_uv = uv();
    let alpha = to_var(Some("alpha"), float(1.0));

    let a = v_uv.x();
    let b = v_uv
        .y()
        .greater_than(float(0.0))
        .select(v_uv.y().sub(float(1.0)), v_uv.y().add(float(1.0)));
    let len2 = a.clone().mul(a).add(b.clone().mul(b));

    let body = if alpha_to_coverage {
        let dlen = to_var(Some("dlen"), fwidth(len2.clone()));
        vec![
            dlen.clone(),
            if_then(
                abs(v_uv.y()).greater_than(float(1.0)),
                vec![alpha
                    .assign(smoothstep(dlen.one_minus(), dlen.add(float(1.0)), len2).one_minus())],
            ),
        ]
    } else {
        vec![if_then(
            abs(v_uv.y()).greater_than(float(1.0)),
            // `len2.greaterThan( 1.0 ).discard()` — the plain form, not
            // `setupDiscard`'s negated `If( cond.not(), … )`, so no `!`.
            vec![if_then(len2.greater_than(float(1.0)), vec![discard()])],
        )]
    };

    let mut statements = vec![alpha.clone()];
    statements.extend(body);
    block(statements, alpha)
}

/// `Line2NodeMaterial.setupDiffuseColor()`'s two additions over
/// `NodeMaterial`'s: the coverage multiply, and the per-end instance colour.
///
/// `vertexColors` does **not** reach the base `vertexColor()` multiply: three's
/// `geometry.hasAttribute( 'color' )` is false on a `LineSegmentsGeometry`
/// (it carries `instanceColorStart` / `instanceColorEnd`), so the colour is
/// selected per end here, in the *fragment* stage, through two varyings.
pub fn setup_diffuse_color(
    alpha_to_coverage: bool,
    vertex_colors: bool,
    attributes: &LineSegmentsAttributes,
    fragment: &mut Vec<NodeRef>,
) {
    fragment.push(
        diffuse_color()
            .w()
            .mul_assign(alpha_line(alpha_to_coverage)),
    );

    if vertex_colors {
        if let Some((color_start, color_end)) = attributes.color_start_end() {
            let instance_color = position_geometry()
                .y()
                .less_than(float(0.5))
                .select(color_start, color_end);
            fragment.push(diffuse_color().rgb().mul_assign(instance_color));
        }
    }
}
