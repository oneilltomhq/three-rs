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
//! * `_useWorldUnits` — both branches: the screen-space quad with round caps,
//!   and the world-space ribbon (`worldStart` / `worldEnd` / `worldPos`
//!   varying properties, `closestLineToLine`) that `webgpu_lines_fat_raycasting`
//!   draws, keyed on
//!   [`Material::world_units`](crate::materials::MeshBasicNodeMaterial::world_units).
//! * `_useAlphaToCoverage` — both branches, keyed on
//!   [`Material::alpha_to_coverage`](crate::materials::MeshBasicNodeMaterial::alpha_to_coverage).
//!
//! Not ported: `_useDash` (the `instanceDistance*` attributes, `lineDistance`,
//! `dashSize` / `gapSize` and the `mod`-discard). No graded frame dashes.

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

/// `closestLineToLine( { p1, p2, p3, p4 } )` — the parameters `( mua, mub )`
/// of the closest points between the lines `p1 p2` and `p3 p4`, each clamped
/// to its segment. A real WGSL `fn`, as three's `Fn` with a layout is.
fn closest_line_to_line() -> Rc<FnDef> {
    shader_fn(
        None,
        vec![
            ("p1", Type::Vec3),
            ("p2", Type::Vec3),
            ("p3", Type::Vec3),
            ("p4", Type::Vec3),
        ],
        Type::Vec2,
        |args| {
            let (p1, p2, p3, p4) = (
                args[0].clone(),
                args[1].clone(),
                args[2].clone(),
                args[3].clone(),
            );
            let p13 = p1.sub(p3.clone());
            let p43 = p4.sub(p3);
            let p21 = p2.sub(p1);

            let d1343 = p13.dot(p43.clone());
            let d4321 = p43.dot(p21.clone());
            let d1321 = p13.dot(p21.clone());
            let d4343 = p43.dot(p43.clone());
            let d2121 = p21.dot(p21.clone());

            let denom = d2121.mul(d4343.clone()).sub(d4321.mul(d4321.clone()));
            let numer = d1343.mul(d4321.clone()).sub(d1321.mul(d4343.clone()));

            let mua = numer.div(denom).saturate();
            let mub = d1343.add(d4321.mul(mua.clone())).div(d4343).saturate();

            vec2_join(vec![mua, mub])
        },
    )
}

/// `varyingProperty( 'vec3', 'worldStart' )` — the segment start in view
/// space ("world" in three's naming), written by the vertex stage.
fn world_start() -> NodeRef {
    varying_property("worldStart", Type::Vec3, false)
}

/// `varyingProperty( 'vec3', 'worldEnd' )`.
fn world_end() -> NodeRef {
    varying_property("worldEnd", Type::Vec3, false)
}

/// `varyingProperty( 'vec4', 'worldPos' )` — the ribbon corner in view space.
fn world_pos() -> NodeRef {
    varying_property("worldPos", Type::Vec4, false)
}

/// `mvpLine`: each segment becomes a quad. In screen space (the default) the
/// quad is built in NDC, widened by `materialLineWidth` screen pixels and
/// extended by half a width at each end for the round caps; with world units
/// it is a view-space box `materialLineWidth` wide, projected.
///
/// Returns the statements to flow into the vertex stage and the clip-space
/// position they leave in the `clip` var.
fn mvp_line(attributes: &LineSegmentsAttributes, world_units: bool) -> (Vec<NodeRef>, NodeRef) {
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

    if world_units {
        statements.push(world_start().assign(start.xyz()));
        statements.push(world_end().assign(end.xyz()));
    }

    // "special case for perspective projection, and segments that terminate
    // either in, or behind, the camera plane" — 4th entry in the 3rd column.
    let perspective = to_const(
        None,
        camera_projection_matrix()
            .element(2)
            .element(3)
            .equal(float(-1.0)),
    );
    let trim = trim_segment_alpha();
    statements.push(if_then(
        perspective.clone(),
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

    if world_units {
        // get the offset direction as perpendicular to the view vector
        let world_dir = end.xyz().sub(start.xyz()).normalize();
        let tmp_fwd = perspective.select(
            mix(start.xyz(), end.xyz(), float(0.5)).normalize(),
            vec3(0.0, 0.0, -1.0),
        );
        let world_up = world_dir.cross(tmp_fwd).normalize();
        let world_fwd = world_dir.cross(world_up.clone());
        let world_pos = world_pos();
        let below_half = || position_geometry().y().less_than(float(0.5));

        statements.push(world_pos.assign(below_half().select(start.clone(), end.clone())));

        // height offset
        let hw = material_line_width().mul(float(0.5));
        statements.push(world_pos.add_assign(vec4_join(vec![
            position_geometry().x().less_than(float(0.0)).select(
                world_up.mul(hw.clone()),
                world_up.mul(hw.clone()).negate(),
            ),
            float(0.0),
        ])));

        // cap extension (`! useDash`; dashes are not ported)
        statements.push(world_pos.add_assign(vec4_join(vec![
            below_half().select(
                world_dir.mul(hw.clone()).negate(),
                world_dir.mul(hw.clone()),
            ),
            float(0.0),
        ])));

        // add width to the box
        statements
            .push(world_pos.add_assign(vec4_join(vec![world_fwd.mul(hw.clone()), float(0.0)])));

        // endcaps
        statements.push(if_then(
            position_geometry()
                .y()
                .greater_than(float(1.0))
                .or(position_geometry().y().less_than(float(0.0))),
            vec![world_pos.sub_assign(vec4_join(vec![
                world_fwd.mul(float(2.0)).mul(hw),
                float(0.0),
            ]))],
        ));

        // project the worldpos
        statements.push(clip.assign(camera_projection_matrix().mul(world_pos)));

        // shift the depth of the projected points so the line segments overlap
        // neatly
        let clip_pose = to_var(None, vec3(0.0, 0.0, 0.0));
        statements.push(clip_pose.clone());
        statements.push(clip_pose.assign(below_half().select(ndc_start, ndc_end)));
        statements.push(clip.z().assign(clip_pose.z().mul(clip.w())));

        return (statements, clip);
    }

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
pub fn setup_position(attributes: &LineSegmentsAttributes, world_units: bool) -> NodeRef {
    let (statements, clip) = mvp_line(attributes, world_units);
    let local = to_var(
        None,
        model_world_matrix_inverse()
            .mul(camera_world_matrix())
            .mul(camera_projection_matrix_inverse())
            .mul(clip),
    );
    position_local().assign(block(statements, local.xyz().div(local.w())))
}

/// `alphaLine` — the coverage of a fat line: the distance to the segment for
/// a world-units line (see [`alpha_line_world_units`]), the round endcaps for
/// a screen-space one.
///
/// The quad runs from `uv.y = -2` to `2` with the segment itself between `-1`
/// and `1`, so `abs( uv.y ) > 1` is inside one of the two caps and the fragment
/// is kept only if it is inside the cap's unit circle.
///
/// With `alphaToCoverage` the circle is antialiased by `fwidth` instead of cut
/// by a `discard`; three also requires `renderer.currentSamples > 0` for that
/// branch, which the port folds into the material flag because a material with
/// `alphaToCoverage` on an unsampled target is not a case any example makes.
fn alpha_line(alpha_to_coverage: bool, world_units: bool) -> NodeRef {
    if world_units {
        return alpha_line_world_units(alpha_to_coverage);
    }
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

/// `alphaLine`'s `useWorldUnits` branch: the view-space distance from the
/// fragment's view ray to the segment, over `materialLineWidth`. Under an
/// orthographic projection the rays are parallel to z, so it is the 2D
/// distance to the segment in view-space xy.
fn alpha_line_world_units(alpha_to_coverage: bool) -> NodeRef {
    let alpha = to_var(Some("alpha"), float(1.0));
    let len = to_var(None, float(0.0));
    let orthographic = to_const(
        None,
        camera_projection_matrix()
            .element(2)
            .element(3)
            .not_equal(float(-1.0)),
    );
    let (world_start, world_end, world_pos) = (world_start(), world_end(), world_pos());

    let ortho = {
        let line_dir = to_const(None, world_end.xy().sub(world_start.xy()));
        let t = world_pos
            .xy()
            .sub(world_start.xy())
            .dot(line_dir.clone())
            .div(line_dir.dot(line_dir.clone()))
            .saturate();
        vec![
            line_dir.clone(),
            len.assign(length(
                world_start.xy().add(line_dir.mul(t)).sub(world_pos.xy()),
            )),
        ]
    };
    let persp = {
        let ray_end = to_const(None, world_pos.xyz().normalize().mul(float(1e5)));
        let line_dir = world_end.clone().sub(world_start.clone());
        let params = to_const(
            None,
            call(
                &closest_line_to_line(),
                vec![
                    world_start.clone(),
                    world_end.clone(),
                    vec3(0.0, 0.0, 0.0),
                    ray_end.clone(),
                ],
            ),
        );
        let p1 = world_start.add(line_dir.mul(params.x()));
        let p2 = ray_end.clone().mul(params.y());
        vec![ray_end, params.clone(), len.assign(length(p1.sub(p2)))]
    };

    let norm = to_const(None, len.div(material_line_width()));
    // `useAlphaToCoverage && renderer.currentSamples > 0`; see `alpha_line`.
    let coverage = if alpha_to_coverage {
        let dnorm = to_const(None, fwidth(norm.clone()));
        vec![
            norm.clone(),
            dnorm.clone(),
            alpha.assign(
                smoothstep(dnorm.negate().add(float(0.5)), dnorm.add(float(0.5)), norm).one_minus(),
            ),
        ]
    } else {
        vec![if_then(norm.greater_than(float(0.5)), vec![discard()])]
    };

    let mut statements = vec![
        alpha.clone(),
        len.clone(),
        orthographic.clone(),
        if_else(orthographic, ortho, persp),
    ];
    statements.extend(coverage);
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
    world_units: bool,
    vertex_colors: bool,
    attributes: &LineSegmentsAttributes,
    fragment: &mut Vec<NodeRef>,
) {
    fragment.push(
        diffuse_color()
            .w()
            .mul_assign(alpha_line(alpha_to_coverage, world_units)),
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
