//! Step 4's gate: one glyph quad, rendered headless, checked against the SDF
//! field of the atlas tile rather than against an image.
//!
//! Nothing here is derived from a screenshot. The expectation is rebuilt from
//! first principles: the quad's world rect and the atlas sub-rect come out of
//! `BatchedText`'s own packed attribute arrays, the SDF value at each pixel
//! centre is a bilinear sample of `VectorFontAtlas::atlas_data()` taken on the
//! CPU, and the shader's `smoothstep( 0.5 - aa, 0.5 + aa, sdf )` is exactly 0
//! below `0.5 - aa` and exactly 1 above `0.5 + aa`. So for every pixel whose
//! CPU-predicted `sdf` is further than `aa` from the 0.5 contour the rendered
//! pixel must be fully the glyph colour or fully the background — no tolerance,
//! no fudge. The band within `aa` of the contour is where `fwidth`'s 2×2-quad
//! derivative lives, and only *that* is skipped.
//!
//! What this proves, each of which is a silent-wrong-output failure mode the
//! plan calls out (§5.2, §5.3, plan §4's capability table):
//!
//! - `R32Float` + `FLOAT32_FILTERABLE` upload works and is **linearly
//!   filtered**: a nearest-sampled atlas would put the contour on texel
//!   boundaries and the 64×64 tile magnified to ~150 px would disagree with the
//!   bilinear prediction over hundreds of pixels.
//! - The atlas is **not** tagged sRGB: an sRGB-decoded `R32Float` would shift
//!   every sample and move the contour.
//! - `position_node` is applied **before** the instance matrix, so the quad
//!   lands on the packed `aGlyphBounds` rect and not at the origin.
//! - The V flip (`1 - uv.y`) matches the Y-down atlas: a flipped sample renders
//!   the glyph upside down, which this comparison sees as a large mismatch.
//! - `fwidth` reaches WGSL as a builtin: without it the shader would not
//!   compile, and with a wrong `aa` the guard band would not contain the
//!   transition.

use std::rc::Rc;

use sdf_text::{BatchedText, BatchedTextOptions, Text, VectorFont};
use three_rs::core::Object3DNode;
use three_rs::{Color, PerspectiveCamera, Renderer, RendererParameters, Scene, Vector3};

const WIDTH: usize = 256;
const HEIGHT: usize = 256;
/// Vertical fov, degrees, and the camera's distance along +z.
const FOV: f64 = 45.0;
const DIST: f64 = 10.0;
const ATLAS_SIZE: u32 = 256;

fn roboto() -> VectorFont {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("sdf-text/tests/assets/Roboto-Regular.ttf");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    VectorFont::parse(bytes, "sdf-text/tests/assets/Roboto-Regular.ttf").unwrap()
}

/// A bilinear sample of the atlas, in the same convention
/// `textureSample( tex, sampler, uv )` uses on a `flip_y = false` upload: uv
/// 0..1 over the image, v measured from the first row of the data, texel centres
/// at `( i + 0.5 ) / size`, and edge clamping.
fn sample_atlas(data: &[f32], size: usize, u: f64, v: f64) -> f64 {
    let x = u * size as f64 - 0.5;
    let y = v * size as f64 - 0.5;
    let x0 = x.floor();
    let y0 = y.floor();
    let fx = x - x0;
    let fy = y - y0;

    let at = |ix: f64, iy: f64| -> f64 {
        let ix = (ix.max(0.0) as usize).min(size - 1);
        let iy = (iy.max(0.0) as usize).min(size - 1);
        data[iy * size + ix] as f64
    };

    let a = at(x0, y0);
    let b = at(x0 + 1.0, y0);
    let c = at(x0, y0 + 1.0);
    let d = at(x0 + 1.0, y0 + 1.0);
    (a * (1.0 - fx) + b * fx) * (1.0 - fy) + (c * (1.0 - fx) + d * fx) * fy
}

#[test]
fn one_glyph_quad_matches_the_atlas_sdf() {
    // --- the batch, on the CPU -------------------------------------------
    let mut batch = BatchedText::new(
        1,
        8,
        BatchedTextOptions {
            // `outlineWidth = 0` collapses `outlineAlpha` onto `fillAlpha`, so
            // `outlineOnly` is zero and `alpha` is the bare fill coverage. The
            // outline band has its own gate below.
            outline_width: 0.0,
            outline_color: None,
            atlas_size: ATLAS_SIZE,
        },
    );
    batch.set_font(Rc::new(roboto()));

    let mut text = Text::new();
    text.set_text("l");
    // `'l'` is one of the two glyphs whose 64×64 tile is bit-exact against the
    // JS golden (its outline is entirely axis-aligned), so the CPU prediction
    // here rests on graded data, not on this port's rasteriser epsilon.
    text.set_font_size(4.0);
    let id = batch.add_text(text);
    assert_eq!(id, 0);
    batch.set_color_at(0, Color::new(1.0, 1.0, 1.0));
    batch.sync();

    assert_eq!(batch.count(), 1, "one glyph, one instance");

    let b = &batch.glyph_bounds_array()[0..4];
    let uvr = &batch.glyph_uv_array()[0..4];
    let (bx0, by0, bx1, by1) = (b[0] as f64, b[1] as f64, b[2] as f64, b[3] as f64);
    let (ru, rv, rw, rh) = (
        uvr[0] as f64,
        uvr[1] as f64,
        uvr[2] as f64,
        uvr[3] as f64,
    );
    assert!(bx1 > bx0 && by1 > by0, "degenerate quad: {b:?}");
    assert!(rw > 0.0 && rh > 0.0, "blank uv rect for a non-space glyph");

    let size = batch.atlas.atlas_size() as usize;
    let atlas: Vec<f32> = batch.atlas.atlas_data().to_vec();

    // --- render -----------------------------------------------------------
    let mut camera = PerspectiveCamera::new(FOV, WIDTH as f64 / HEIGHT as f64, 0.1, 100.0);
    camera.object.position.set(0.0, 0.0, DIST);
    camera.look_at(&Vector3::ZERO);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));
    scene.add(batch.node());
    batch.node().update_matrix_world(true);

    let mut renderer = Renderer::new(RendererParameters { antialias: false });
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(WIDTH as f64, HEIGHT as f64);
    renderer.render(&mut scene, &mut camera);
    let (w, h, pixels) = renderer.read_canvas_pixels();
    assert_eq!((w as usize, h as usize), (WIDTH, HEIGHT));

    // --- predict ----------------------------------------------------------
    // The member matrix is the identity and the quad lies in the z = 0 plane, so
    // with the camera on +z looking at the origin the projection is the plain
    // perspective divide: `ndc.x = x / ( d · aspect · tan( fov / 2 ) )`.
    let aspect = WIDTH as f64 / HEIGHT as f64;
    let t = (FOV * std::f64::consts::PI / 360.0).tan();
    let half_w = DIST * t * aspect;
    let half_h = DIST * t;
    let world_at = |px: f64, py: f64| -> (f64, f64) {
        let ndc_x = 2.0 * px / WIDTH as f64 - 1.0;
        let ndc_y = 1.0 - 2.0 * py / HEIGHT as f64;
        (ndc_x * half_w, ndc_y * half_h)
    };

    // The predicted SDF value at a pixel centre, or `None` when the pixel centre
    // falls outside the quad (nothing is drawn there).
    let predict = |i: usize, j: usize| -> Option<f64> {
        let (x, y) = world_at(i as f64 + 0.5, j as f64 + 0.5);
        if x < bx0 || x > bx1 || y < by0 || y > by1 {
            return None;
        }
        let st_x = (x - bx0) / (bx1 - bx0);
        let st_y = (y - by0) / (by1 - by0);
        // `atlasUV = vec2( r.x + st.x · r.z, r.y + ( 1 - st.y ) · r.w )`.
        let u = ru + st_x * rw;
        let v = rv + (1.0 - st_y) * rh;
        Some(sample_atlas(&atlas, size, u, v))
    };

    let mut field = vec![f64::NAN; WIDTH * HEIGHT];
    for j in 0..HEIGHT {
        for i in 0..WIDTH {
            if let Some(s) = predict(i, j) {
                field[j * WIDTH + i] = s;
            }
        }
    }

    // --- compare ----------------------------------------------------------
    let mut fill = 0usize;
    let mut background = 0usize;
    let mut skipped = 0usize;
    let mut wrong: Vec<(usize, usize, f64, u8)> = Vec::new();

    for j in 0..HEIGHT {
        for i in 0..WIDTH {
            let idx = (j * WIDTH + i) * 4;
            let got = pixels[idx]; // white-on-black, so one channel is enough
            let s = field[j * WIDTH + i];

            if s.is_nan() {
                // Outside the quad. A pixel next to the quad edge can still be
                // touched by rasterisation coverage at the boundary, so only
                // pixels a clear pixel away from it are asserted on.
                let inside_neighbour = (-1i64..=1)
                    .flat_map(|dj| (-1i64..=1).map(move |di| (di, dj)))
                    .any(|(di, dj)| {
                        let ni = i as i64 + di;
                        let nj = j as i64 + dj;
                        ni >= 0
                            && nj >= 0
                            && (ni as usize) < WIDTH
                            && (nj as usize) < HEIGHT
                            && !field[nj as usize * WIDTH + ni as usize].is_nan()
                    });
                if inside_neighbour {
                    skipped += 1;
                    continue;
                }
                background += 1;
                if got > 5 {
                    wrong.push((i, j, f64::NAN, got));
                }
                continue;
            }

            // `fwidth( sdf ) = |dpdx| + |dpdy|`, and `aa = fwidth · 0.5`, so the
            // full transition spans `fwidth` either side of nothing — it is
            // `[ 0.5 - aa, 0.5 + aa ]`. The finite difference over neighbouring
            // pixel centres is the same quantity the GPU's 2×2 quad takes, up to
            // which pixel of the quad is the base, so the guard is doubled.
            let grad = |di: i64, dj: i64| -> f64 {
                let ni = (i as i64 + di).clamp(0, WIDTH as i64 - 1) as usize;
                let nj = (j as i64 + dj).clamp(0, HEIGHT as i64 - 1) as usize;
                let n = field[nj * WIDTH + ni];
                if n.is_nan() {
                    0.0
                } else {
                    (n - s).abs()
                }
            };
            let fwidth = grad(1, 0).max(grad(-1, 0)) + grad(0, 1).max(grad(0, -1));
            let guard = fwidth + 1e-6;

            if s > 0.5 + guard {
                fill += 1;
                if got < 250 {
                    wrong.push((i, j, s, got));
                }
            } else if s < 0.5 - guard {
                background += 1;
                if got > 5 {
                    wrong.push((i, j, s, got));
                }
            } else {
                skipped += 1;
            }
        }
    }

    println!(
        "glyph 'l': quad [{bx0:.4} {by0:.4} {bx1:.4} {by1:.4}] uv [{ru:.5} {rv:.5} {rw:.5} {rh:.5}]"
    );
    println!("fill {fill}, background {background}, transition band {skipped}, wrong {}", wrong.len());

    // Not vacuous: the glyph has to actually cover pixels, and the transition
    // band has to be a thin rim rather than the whole quad.
    assert!(fill > 400, "only {fill} fully-covered pixels — is the glyph drawn at all?");
    assert!(
        background > WIDTH * HEIGHT / 2,
        "only {background} background pixels"
    );
    assert!(
        skipped < fill,
        "the anti-aliased band ({skipped}) is wider than the fill ({fill}) — `aa` is wrong"
    );

    if !wrong.is_empty() {
        for (i, j, s, got) in wrong.iter().take(20) {
            println!("  ({i},{j}) predicted sdf {s:.4} → rendered {got}");
        }
        panic!(
            "{} of {} classified pixels disagree with the atlas SDF",
            wrong.len(),
            fill + background
        );
    }
}

/// The outline/halo band: with `outlineWidth = 0.1` and a halo colour, the
/// pixels whose predicted SDF sits in `( 0.5 - 0.1, 0.5 )` must be the halo
/// colour and the ones above 0.5 the fill colour — the `outlineOnly = max(
/// outlineAlpha - fillAlpha, 0 )` branch, which the step-4 capability table
/// needs and which no rung exercises.
#[test]
fn the_outline_band_is_the_halo_colour() {
    const OUTLINE: f64 = 0.1;

    let mut batch = BatchedText::new(
        1,
        8,
        BatchedTextOptions {
            outline_width: OUTLINE,
            // Pure red halo, pure green fill: two channels that cannot be
            // confused with each other or with the black background.
            outline_color: Some(Color::new(1.0, 0.0, 0.0)),
            atlas_size: ATLAS_SIZE,
        },
    );
    batch.set_font(Rc::new(roboto()));

    let mut text = Text::new();
    text.set_text("l");
    text.set_font_size(4.0);
    batch.add_text(text);
    batch.set_color_at(0, Color::new(0.0, 1.0, 0.0));
    batch.sync();

    let b = &batch.glyph_bounds_array()[0..4];
    let uvr = &batch.glyph_uv_array()[0..4];
    let (bx0, by0, bx1, by1) = (b[0] as f64, b[1] as f64, b[2] as f64, b[3] as f64);
    let (ru, rv, rw, rh) = (
        uvr[0] as f64,
        uvr[1] as f64,
        uvr[2] as f64,
        uvr[3] as f64,
    );
    let size = batch.atlas.atlas_size() as usize;
    let atlas: Vec<f32> = batch.atlas.atlas_data().to_vec();

    let mut camera = PerspectiveCamera::new(FOV, WIDTH as f64 / HEIGHT as f64, 0.1, 100.0);
    camera.object.position.set(0.0, 0.0, DIST);
    camera.look_at(&Vector3::ZERO);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));
    scene.add(batch.node());
    batch.node().update_matrix_world(true);

    let mut renderer = Renderer::new(RendererParameters { antialias: false });
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(WIDTH as f64, HEIGHT as f64);
    renderer.render(&mut scene, &mut camera);
    let (_, _, pixels) = renderer.read_canvas_pixels();

    let aspect = WIDTH as f64 / HEIGHT as f64;
    let t = (FOV * std::f64::consts::PI / 360.0).tan();
    let half_w = DIST * t * aspect;
    let half_h = DIST * t;

    let mut halo = 0usize;
    let mut core = 0usize;
    let mut wrong = Vec::new();

    for j in 0..HEIGHT {
        for i in 0..WIDTH {
            let ndc_x = 2.0 * (i as f64 + 0.5) / WIDTH as f64 - 1.0;
            let ndc_y = 1.0 - 2.0 * (j as f64 + 0.5) / HEIGHT as f64;
            let (x, y) = (ndc_x * half_w, ndc_y * half_h);
            if x < bx0 || x > bx1 || y < by0 || y > by1 {
                continue;
            }
            let st_x = (x - bx0) / (bx1 - bx0);
            let st_y = (y - by0) / (by1 - by0);
            let s = sample_atlas(&atlas, size, ru + st_x * rw, rv + (1.0 - st_y) * rh);

            let idx = (j * WIDTH + i) * 4;
            let (r, g) = (pixels[idx], pixels[idx + 1]);

            // A generous margin either side of both contours keeps every
            // anti-aliased pixel out: only the clear interior of each band is
            // asserted on.
            if s > 0.5 + 0.03 {
                core += 1;
                if !(g > 250 && r < 60) {
                    wrong.push((i, j, s, r, g, "fill"));
                }
            } else if s > 0.5 - OUTLINE + 0.03 && s < 0.5 - 0.03 {
                halo += 1;
                if !(r > 250 && g < 60) {
                    wrong.push((i, j, s, r, g, "halo"));
                }
            }
        }
    }

    println!("outline band: {halo} halo pixels, {core} fill pixels, {} wrong", wrong.len());
    assert!(halo > 100, "only {halo} halo pixels — the outline band is missing");
    assert!(core > 400, "only {core} fill pixels");
    if !wrong.is_empty() {
        for (i, j, s, r, g, which) in wrong.iter().take(20) {
            println!("  ({i},{j}) sdf {s:.4} expected {which}, got r={r} g={g}");
        }
        panic!("{} pixels in the wrong band", wrong.len());
    }
}

/// `'l'` is very nearly symmetric under a vertical flip, so the test above would
/// survive a wrong `1 - uv.y`. `'L'` is not symmetric under either flip — stem
/// on the left, bar at the bottom — so the centroid of the rendered ink inside
/// the quad pins both axes of the atlas sampling at once.
#[test]
fn the_atlas_v_flip_and_u_direction_are_right() {
    let mut batch = BatchedText::new(
        1,
        8,
        BatchedTextOptions {
            outline_width: 0.0,
            outline_color: None,
            atlas_size: ATLAS_SIZE,
        },
    );
    batch.set_font(Rc::new(roboto()));

    let mut text = Text::new();
    text.set_text("L");
    text.set_font_size(4.0);
    batch.add_text(text);
    batch.set_color_at(0, Color::new(1.0, 1.0, 1.0));
    batch.sync();

    let b = &batch.glyph_bounds_array()[0..4];
    let (bx0, by0, bx1, by1) = (b[0] as f64, b[1] as f64, b[2] as f64, b[3] as f64);

    let mut camera = PerspectiveCamera::new(FOV, WIDTH as f64 / HEIGHT as f64, 0.1, 100.0);
    camera.object.position.set(0.0, 0.0, DIST);
    camera.look_at(&Vector3::ZERO);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));
    scene.add(batch.node());
    batch.node().update_matrix_world(true);

    let mut renderer = Renderer::new(RendererParameters { antialias: false });
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(WIDTH as f64, HEIGHT as f64);
    renderer.render(&mut scene, &mut camera);
    let (_, _, pixels) = renderer.read_canvas_pixels();

    let aspect = WIDTH as f64 / HEIGHT as f64;
    let t = (FOV * std::f64::consts::PI / 360.0).tan();
    let half_w = DIST * t * aspect;
    let half_h = DIST * t;

    // The ink centroid, in the quad's own normalised (s, t) coordinates, where
    // t = 0 is the quad's bottom edge in world space.
    let mut sum_s = 0.0;
    let mut sum_t = 0.0;
    let mut weight = 0.0;
    for j in 0..HEIGHT {
        for i in 0..WIDTH {
            let ndc_x = 2.0 * (i as f64 + 0.5) / WIDTH as f64 - 1.0;
            let ndc_y = 1.0 - 2.0 * (j as f64 + 0.5) / HEIGHT as f64;
            let (x, y) = (ndc_x * half_w, ndc_y * half_h);
            if x < bx0 || x > bx1 || y < by0 || y > by1 {
                continue;
            }
            let v = pixels[(j * WIDTH + i) * 4] as f64 / 255.0;
            if v <= 0.5 {
                continue;
            }
            sum_s += v * (x - bx0) / (bx1 - bx0);
            sum_t += v * (y - by0) / (by1 - by0);
            weight += v;
        }
    }
    assert!(weight > 100.0, "almost no ink ({weight:.1}) — the glyph is missing");
    let (cs, ct) = (sum_s / weight, sum_t / weight);
    println!("'L' ink centroid in quad space: s {cs:.4}, t {ct:.4}");

    // Roboto's 'L': a full-height stem on the left plus a bar across the bottom.
    // Ink therefore sits left of centre and below it. A wrong `1 - uv.y` mirrors
    // `t` about 0.5; a wrong U direction mirrors `s`.
    assert!(ct < 0.45, "ink centroid is not in the lower half (t = {ct:.4}) — the V flip is wrong");
    assert!(cs < 0.45, "ink centroid is not in the left half (s = {cs:.4}) — the U direction is wrong");
}
