//! The numeric oracle for `webgpu_materials`, from three.js r186's own `src/`
//! run in node with no GPU (`scouts/webgpu_materials/oracle.mjs`).
//!
//! Everything the graded frame depends on that is *not* a pixel: the 51
//! deterministic `Math.random()` draws the page makes, the seventeen mesh
//! transforms after the single `animate()` step, the camera's three matrices,
//! and `GridHelper`'s two vertex buffers. Rung 5 lost 2.41% of its pixels to a
//! Euler-written-without-syncing-the-quaternion bug of exactly this class, and
//! the image is the most expensive possible way to find it.

use serde_json::Value;

use three_rs::core::Object3D;
use three_rs::helpers::GridHelper;
use three_rs::math::{Color, Vector3};
use three_rs::testing::DeterministicRandom;
use three_rs::PerspectiveCamera;

const INNER_WIDTH: f64 = 800.0;
const INNER_HEIGHT: f64 = 500.0;

fn oracle() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/webgpu_materials/scene_t0.json");
    serde_json::from_str(&std::fs::read_to_string(&path).expect("the oracle fixture is readable"))
        .expect("the oracle fixture is JSON")
}

fn floats(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .expect("an array of numbers")
        .iter()
        .map(|v| v.as_f64().expect("a number"))
        .collect()
}

fn assert_close(label: &str, actual: &[f64], expected: &[f64], tol: f64) {
    assert_eq!(actual.len(), expected.len(), "{label}: length");
    for (i, (a, e)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (a - e).abs() <= tol,
            "{label}[{i}]: {a} vs {e} (delta {})",
            (a - e).abs()
        );
    }
}

/// The 51 draws `addMesh()` makes, three per mesh, in material-creation order.
#[test]
fn deterministic_random_draws() {
    let oracle = oracle();
    let expected = floats(&oracle["randomDraws"]);
    assert_eq!(expected.len(), 51);
    assert_eq!(
        oracle["randomDrawCount"].as_u64(),
        Some(51),
        "the page draws three randoms for each of seventeen meshes and nothing else"
    );

    let mut random = DeterministicRandom::new();
    let actual: Vec<f64> = (0..51).map(|_| random.next()).collect();
    // `x = sin( seed ) * 10000; return x - floor( x )`. Rust's `f64::sin` and
    // V8's differ by at most an ulp, and the multiply by 10000 turns that into
    // an ulp of a number near 10000, i.e. ~2e-12 in the fraction. The draws are
    // spread over [0,1), so 1e-9 still pins the sequence exactly while leaving
    // the libm difference alone. Every rung on the ladder shares it.
    assert_close("randomDraws", &actual, &expected, 1e-9);
}

/// The seventeen meshes' transforms *after* the single `animate()` step, which
/// adds `0.01` to `rotation.x` and `0.005` to `rotation.y` on top of the random
/// Euler. Writing the Euler without re-deriving the quaternion leaves
/// `matrixWorld` at the pre-step rotation and the whole scene subtly wrong.
#[test]
fn mesh_transforms_after_one_animate_step() {
    let oracle = oracle();
    let expected = oracle["meshes"].as_array().expect("seventeen meshes");
    assert_eq!(expected.len(), 17);

    let mut random = DeterministicRandom::new();
    let mut objects: Vec<Object3D> = Vec::new();
    for i in 0..17 {
        let mut object = Object3D::default();
        object.position.x = (i % 4) as f64 * 200.0 - 400.0;
        object.position.z = (i / 4) as f64 * 200.0 - 200.0;
        object.set_rotation(
            random.next() * 200.0 - 100.0,
            random.next() * 200.0 - 100.0,
            random.next() * 200.0 - 100.0,
        );
        objects.push(object);
    }

    // `animate()`: `object.rotation.x += 0.01; object.rotation.y += 0.005`.
    for object in &mut objects {
        let (x, y, z) = (object.rotation.x, object.rotation.y, object.rotation.z);
        object.set_rotation(x + 0.01, y + 0.005, z);
    }

    for (i, object) in objects.iter_mut().enumerate() {
        object.update_matrix_world(None);
        let want = &expected[i];
        assert_eq!(want["index"].as_u64(), Some(i as u64));

        assert_close(
            &format!("mesh {i} position"),
            &[object.position.x, object.position.y, object.position.z],
            &floats(&want["position"]),
            0.0,
        );
        assert_close(
            &format!("mesh {i} rotation"),
            &[object.rotation.x, object.rotation.y, object.rotation.z],
            &floats(&want["rotation"]),
            // The random draw's ~2e-12 libm residue, times the page's 200.
            1e-8,
        );
        assert_close(
            &format!("mesh {i} quaternion"),
            &[
                object.quaternion.x,
                object.quaternion.y,
                object.quaternion.z,
                object.quaternion.w,
            ],
            &floats(&want["quaternion"]),
            1e-8,
        );
        assert_close(
            &format!("mesh {i} matrixWorld"),
            &object.matrix_world.elements,
            &floats(&want["matrixWorld"]),
            1e-8,
        );
    }
}

/// `animate()` moves the camera to `( cos( 0 ) * 1000, 200, sin( 0 ) * 1000 )`
/// and calls `lookAt( scene.position )`, and the graded frame is rendered from
/// the matrices that produces.
#[test]
fn camera_matrices_after_one_animate_step() {
    let oracle = oracle();
    let want = &oracle["camera"];

    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 2000.0);
    {
        let mut object = camera.node.borrow_mut();
        object.position.set(0.0, 200.0, 800.0);
        // `const timer = 0.0001 * Date.now()` with the harness' frozen clock.
        let timer = 0.0f64;
        object.position.x = timer.cos() * 1000.0;
        object.position.z = timer.sin() * 1000.0;
    }
    camera.look_at(&Vector3::ZERO);
    camera.update_matrix_world();
    camera.update_projection_matrix();

    let position = camera.node.borrow().position;
    assert_close(
        "camera position",
        &[position.x, position.y, position.z],
        &floats(&want["position"]),
        1e-12,
    );
    assert_close(
        "camera matrixWorld",
        &camera.node.borrow().matrix_world.elements,
        &floats(&want["matrixWorld"]),
        1e-12,
    );
    assert_close(
        "camera matrixWorldInverse",
        &camera.matrix_world_inverse.elements,
        &floats(&want["matrixWorldInverse"]),
        1e-12,
    );
    // The oracle ran three.js' `src/` directly, so its camera kept the default
    // `WebGLCoordinateSystem` and its projection matrix maps z to [-1,1];
    // `WebGPURenderer` sets `WebGPUCoordinateSystem` and maps z to [0,1]. The
    // two differ in exactly two elements, and algebraically:
    //
    //   m[10]: -( f + n ) / ( f - n )  →  -f / ( f - n )     = ( m[10] - 1 ) / 2
    //   m[14]: -2 f n / ( f - n )      →  -f n / ( f - n )   =   m[14] / 2
    //
    // so the oracle still grades the port's matrix, through that conversion.
    let mut expected_projection = floats(&want["projectionMatrix"]);
    expected_projection[10] = (expected_projection[10] - 1.0) / 2.0;
    expected_projection[14] /= 2.0;
    assert_close(
        "camera projectionMatrix (WebGPU clip space)",
        &camera.projection_matrix.elements,
        &expected_projection,
        1e-12,
    );
}

/// `new GridHelper( 1000, 40, 0x303030, 0x303030 )` — 82 segments, 164
/// vertices, and one colour throughout because the page passes the centre and
/// grid colours the same.
#[test]
fn grid_helper_buffers() {
    let oracle = oracle();
    let want = &oracle["gridHelper"];

    let (geometry, material) = GridHelper::parts(
        1000.0,
        40,
        Color::from_hex(0x303030),
        Color::from_hex(0x303030),
    );
    assert!(material.vertex_colors);

    let position = geometry
        .get_attribute("position")
        .expect("the helper sets a position attribute");
    let color = geometry
        .get_attribute("color")
        .expect("the helper sets a color attribute");

    let positions: Vec<f64> = position.array().iter().map(|v| *v as f64).collect();
    let colors: Vec<f64> = color.array().iter().map(|v| *v as f64).collect();

    assert_eq!(
        positions.len() / 3,
        want["vertexCount"].as_u64().unwrap() as usize
    );
    assert_eq!(colors.len(), positions.len());

    assert_close(
        "gridHelper positionFirst12",
        &positions[..12],
        &floats(&want["positionFirst12"]),
        0.0,
    );
    assert_close(
        "gridHelper positionLast12",
        &positions[positions.len() - 12..],
        &floats(&want["positionLast12"]),
        0.0,
    );
    assert_close(
        "gridHelper colorFirst12",
        &colors[..12],
        &floats(&want["colorFirst12"]),
        1e-7,
    );

    // `0x303030` twice, so every one of the 492 colour floats is the same
    // linear value — the grid is one hue and the centre line does not show.
    let distinct = floats(&want["colorDistinct"]);
    assert_eq!(distinct.len(), 1);
    for (i, c) in colors.iter().enumerate() {
        assert!(
            (c - distinct[0]).abs() <= 1e-7,
            "gridHelper color[{i}]: {c} vs {}",
            distinct[0]
        );
    }

    let sum: f64 = positions.iter().sum();
    assert!(
        (sum - want["positionSum"].as_f64().unwrap()).abs() <= 1e-9,
        "gridHelper positionSum: {sum}"
    );
}

/// The page's last material is a `Loop()` used as a `colorNode`, and three
/// renders that teapot opaque black.
///
/// `LoopNode.generate()` writes the `for` into the flow and returns an *empty*
/// snippet, so `vec4( colorNode )` in `setupDiffuseColor()` casts nothing:
/// `DiffuseColor = vec4<f32>(  );`, two spaces, a zero vector, and the four
/// texture taps the loop ran are thrown away. **The port reproduces this
/// deliberately** — `docs/nodes.md` §8 — because the graded frame contains the
/// black teapot and a port that "fixed" the bug would lose those pixels.
///
/// Pinned here rather than left to the image: the difference between the bug
/// and the fix is one teapot out of seventeen, which the 0.1% threshold would
/// very nearly absorb.
#[test]
fn a_loop_as_a_color_node_discards_its_result() {
    use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext};
    use three_rs::nodes::tsl::{
        float, loop_index, loop_statement, osc_sine, texture_uv, time, to_var, uv, vec2_join, vec4,
    };
    use three_rs::nodes::{NodeBuilder, NodeRef, Type};

    let map = three_rs::Texture::new(4, 4, Some(vec![0; 4]));
    let i = loop_index();
    let output = to_var(None, vec4(0.0, 0.0, 0.0, 1.0));
    let scale_i = osc_sine(time()).mul(0.09).mul(i.to(Type::F32));
    let scale_i_neg = to_var(None, scale_i.clone().negate());
    let tap = |offset: NodeRef| output.assign(output.add(texture_uv(&map, uv().add(offset))));

    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(loop_statement(
        10,
        i,
        vec![
            tap(vec2_join(vec![scale_i.clone(), float(0.0)])),
            tap(vec2_join(vec![scale_i_neg.clone(), float(0.0)])),
            tap(vec2_join(vec![float(0.0), scale_i])),
            tap(vec2_join(vec![float(0.0), scale_i_neg])),
        ],
    ));

    let flow = setup(&material, &SetupContext::default(), None);
    let wgsl = NodeBuilder::new().build(&flow).fragment_wgsl;

    assert!(
        wgsl.contains("\tfor ( var i : i32 = 0; i < 10; i ++ ) {"),
        "the loop still runs:\n{wgsl}"
    );
    assert!(
        wgsl.contains("\tDiffuseColor = vec4<f32>(  );"),
        "the loop's value is an empty snippet, cast to nothing:\n{wgsl}"
    );
}
