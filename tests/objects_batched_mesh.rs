//! CPU-side gates for `BatchedMesh` and `webgpu_mesh_batch`, from the scout's
//! `dump.json` for three.js r186 (`handoff/scouts/rung11/`). None of this
//! touches the GPU: it is the data the node system and the backend read.

use three_rs::math::Matrix4;
use three_rs::objects::BatchCamera;
use three_rs::RenderCamera;

#[path = "../examples/webgpu_mesh_batch.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_mesh_batch;

fn assert_close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < 1e-5,
        "{what}: {actual} != {expected}"
    );
}

#[test]
fn batched_mesh_matches_threes_state() {
    let (_scene, mut camera, mesh, rotation_speeds, ids) = webgpu_mesh_batch::build();
    webgpu_mesh_batch::animate_meshes(&mesh, &ids, &rotation_speeds);

    let mut object = mesh.borrow_mut();
    let batched = object.payload.batched_mesh_mut().unwrap();

    // `_initMatricesTexture` / `_initIndirectTexture` / `_initColorsTexture`
    // for `maxInstanceCount = 512`.
    let entry = batched.batch_entry();
    assert_eq!(entry.matrices.size(), (48, 48), "matrices texture");
    assert_eq!(entry.indirect.size(), (23, 23), "indirect texture");
    assert_eq!(
        entry.colors.as_ref().unwrap().size(),
        (23, 23),
        "colours texture"
    );

    // The three geometries' packed index ranges: cone, box, sphere.
    for (id, start, count) in [(0, 0, 192), (1, 192, 36), (2, 228, 672)] {
        let range = batched.geometry_range_at(id).unwrap();
        assert_eq!((range.start, range.count), (start, count), "range {id}");
    }

    // Instance 0's matrix after one `animateMeshes()` step.
    #[rustfmt::skip]
    let expected: [f64; 16] = [
        -0.579944, 0.209340, -0.199118, 0.0,
        0.225518, 0.607128, -0.018541, 0.0,
        0.180590, -0.085901, -0.616291, 0.0,
        -12.960941, 6.530896, -0.369800, 1.0,
    ];
    let matrix = batched.matrix_at(ids[0]);
    for (i, want) in expected.iter().enumerate() {
        assert_close(matrix.elements[i], *want, &format!("matrix[{i}]"));
    }

    // `setColorAt` writes linear rgb; `Color.toArray` writes only rgb, so the
    // alpha stays at the texture's init fill of 1 — reproduced deliberately.
    let colors = entry.colors.as_ref().unwrap();
    let texel = |i: usize| {
        colors.with_f32(|data| {
            [
                data[i * 4],
                data[i * 4 + 1],
                data[i * 4 + 2],
                data[i * 4 + 3],
            ]
        })
    };
    for (i, want) in [
        (0usize, [0.617207f32, 0.009721, 0.194618, 1.0]),
        (1, [0.341914, 0.376262, 0.806952, 1.0]),
    ] {
        let got = texel(i);
        for c in 0..4 {
            assert_close(got[c] as f64, want[c] as f64, &format!("colour {i}[{c}]"));
        }
    }

    // `onBeforeRender()`: the per-instance frustum cull keeps 453 of the 512
    // instances, and the custom radix sort puts them in this order.
    camera.update_matrix_world();
    let batch_camera = BatchCamera {
        projection_matrix: camera.projection_matrix(),
        matrix_world_inverse: camera.matrix_world_inverse(),
        matrix_world: camera.matrix_world(),
        coordinate_system: camera.coordinate_system(),
        far: camera.far,
    };
    batched.on_before_render(&Matrix4::identity(), &batch_camera);

    assert_eq!(batched.multi_draw_count(), 453, "visible instances");
    assert_eq!(
        batched.indirect_prefix(12),
        vec![270, 8, 497, 508, 383, 0, 254, 251, 416, 509, 64, 211],
        "the sorted draw list's first twelve instance ids"
    );
    assert_eq!(batched.sub_draws().len(), 453, "drawIndexed() calls");
}
