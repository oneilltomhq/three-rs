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
    let (_scene, mut camera, mut controls, mesh, rotation_speeds, ids) = webgpu_mesh_batch::build();
    webgpu_mesh_batch::animate_meshes(&mesh, &ids, &rotation_speeds);
    // The page's `animate()` turns the camera with `controls.update()` before it
    // renders, so the frame this gates is rendered from the rotated pose.
    controls.update(&mut camera, None);

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

// `BatchedMesh.raycast()`: one `Mesh.raycast()` per instance over its
// geometry's range, tagged with the `batchId`. Expected values are three.js'
// for the same batch, run under node.
#[test]
fn raycast() {
    use three_rs::core::Raycaster;
    use three_rs::geometries::{box_geometry, plane_geometry};
    use three_rs::materials::MeshBasicNodeMaterial;
    use three_rs::math::Vector3;
    use three_rs::objects::BatchedMesh;

    let mesh = BatchedMesh::new(3, 100, 100, MeshBasicNodeMaterial::default());
    {
        let mut object = mesh.borrow_mut();
        let batched = object.payload.batched_mesh_mut().unwrap();
        let cube = batched.add_geometry(&box_geometry(1.0, 1.0, 1.0, 1, 1, 1));
        let plane = batched.add_geometry(&plane_geometry(1.0, 1.0, 1, 1));
        let ids = [
            batched.add_instance(cube),
            batched.add_instance(plane),
            batched.add_instance(cube),
        ];
        for (i, id) in ids.into_iter().enumerate() {
            let mut matrix = Matrix4::identity();
            matrix.make_translation(2.0 * i as f64 - 2.0, 0.0, 0.0);
            batched.set_matrix_at(id, &matrix);
        }
    }
    mesh.update_matrix_world(false);

    for (x, batch_id, distance, face_index, face) in [
        (2.0, 2, 4.5, 8, (16, 18, 17)),
        (0.0, 1, 5.0, 12, (24, 26, 25)),
        (-2.0, 0, 4.5, 8, (16, 18, 17)),
    ] {
        let raycaster = Raycaster::new(
            Vector3::new(x, 0.1, 5.0),
            Vector3::new(0.0, 0.0, -1.0),
            0.0,
            f64::INFINITY,
        );
        let hits = raycaster.intersect_object(&mesh, false);
        assert_eq!(hits.len(), 1, "x = {x}");
        let hit = &hits[0];
        assert_eq!(hit.batch_id, Some(batch_id));
        assert_eq!(hit.distance, distance);
        assert_eq!(hit.face_index, Some(face_index));
        let f = hit.face.unwrap();
        assert_eq!((f.a, f.b, f.c), face);
        let uv = hit.uv.unwrap();
        assert!((uv.x - 0.5).abs() < 1e-12 && (uv.y - 0.6).abs() < 1e-12);
    }
}
