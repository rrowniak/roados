//! The model file's shape, exercised without a filesystem.
//!
//! `AGENTS.md` permits no test that needs a filesystem, and the resolution is
//! the API shape: [`load_from_bytes`](ui_core::render::meshio::load_from_bytes)
//! is the whole of the loader and `load_from_path` is a wrapper over it, so a
//! test holds the bytes in memory — here via `include_bytes!`, the crate's
//! first, over a committed fixture.
//!
//! What these tests prove, and what they do not: they prove the **reader**
//! against a file of the real asset's exact shape and counts (2_032 triangles,
//! five named sub-meshes, baked node translations). **They prove nothing about
//! the converter**, which does not exist in this task and is task 39's: the
//! fixture is built by an offline generator from figures read out of
//! `sedan.glb`, so a converter that mis-bakes a transform is invisible here and
//! will be caught by 39 writing the same assertions against the real
//! `sedan.roados` — which is why the fixture lives under `tests/data/` and not
//! in the demo's `assets/`: it is a test input, and putting it where the
//! runtime looks would put a car on screen, which is 39's capture.

use ui_core::render::mesh::Mesh;
use ui_core::render::meshio::load_from_bytes;

/// The committed fixture, held in memory: no filesystem, no wall clock.
fn sedan() -> Mesh {
    let bytes = include_bytes!("data/sedan.roados");
    load_from_bytes(&bytes[..]).expect("the sedan fixture loads")
}

#[test]
fn a_mesh_of_the_sedans_shape_loads_and_its_wheels_are_outside_its_body() {
    let mesh = sedan();

    // The verified counts: 2_032 triangles, five sub-meshes with 704 and 332
    // triangles respectively.
    assert_eq!(mesh.indices.len(), 6096);
    assert_eq!(mesh.sub_meshes.len(), 5);
    let names: Vec<&str> = mesh
        .sub_meshes
        .iter()
        .map(|sub| sub.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "body",
            "wheel-front-left",
            "wheel-front-right",
            "wheel-back-left",
            "wheel-back-right"
        ],
        "the five names in file order"
    );
    let tris: Vec<u32> = mesh
        .sub_meshes
        .iter()
        .map(|sub| sub.index_count / 3)
        .collect();
    assert_eq!(tris, [704, 332, 332, 332, 332]);

    // The ranges tile the index array exactly and in order.
    let mut running: u32 = 0;
    for sub in &mesh.sub_meshes {
        assert_eq!(sub.first_index, running);
        running += sub.index_count;
    }
    assert_eq!(running, mesh.indices.len() as u32);

    // Every index names a vertex that exists.
    for (pos, index) in mesh.indices.iter().enumerate() {
        assert!(
            (*index as usize) < mesh.vertices.len(),
            "index {index} at position {pos} past {} vertices",
            mesh.vertices.len()
        );
    }

    // Every normal is unit length: the file's normals arrive normalised
    // through `MeshVertex::new`, and a corrupt one would surface here.
    for vertex in &mesh.vertices {
        let len = (vertex.normal[0] * vertex.normal[0]
            + vertex.normal[1] * vertex.normal[1]
            + vertex.normal[2] * vertex.normal[2])
            .sqrt();
        assert!(
            (len - 1.0).abs() < 1e-5,
            "a normal of length {len} is not a direction"
        );
    }

    // The baked-transform requirement, which is what makes it load-bearing
    // rather than asserted: the four wheel sub-meshes sit in four distinct
    // places — their centroids pairwise more than 0.1 m apart — and each
    // wheel's extent reaches outside the body's bounding box in at least one
    // axis (here below it: the wheels dip to y -0.05 against the body's 0.15).
    // A reader that added a base to the indices, or a converter that forgot to
    // bake a node translation, puts all four wheels at the origin inside the
    // body, and this is the test that sees it.
    let body = &mesh.sub_meshes[0];
    let mut body_min = [f32::INFINITY; 3];
    let mut body_max = [f32::NEG_INFINITY; 3];
    for index in
        &mesh.indices[body.first_index as usize..(body.first_index + body.index_count) as usize]
    {
        let pos = mesh.vertices[*index as usize].position;
        for axis in 0..3 {
            body_min[axis] = body_min[axis].min(pos[axis]);
            body_max[axis] = body_max[axis].max(pos[axis]);
        }
    }
    let mut centroids = Vec::new();
    for sub in mesh.sub_meshes.iter().skip(1) {
        let mut sum = [0.0f64; 3];
        let mut count = 0u32;
        let mut wheel_min = [f32::INFINITY; 3];
        let mut wheel_max = [f32::NEG_INFINITY; 3];
        for index in
            &mesh.indices[sub.first_index as usize..(sub.first_index + sub.index_count) as usize]
        {
            let pos = mesh.vertices[*index as usize].position;
            for axis in 0..3 {
                sum[axis] += f64::from(pos[axis]);
                wheel_min[axis] = wheel_min[axis].min(pos[axis]);
                wheel_max[axis] = wheel_max[axis].max(pos[axis]);
            }
            count += 1;
        }
        let centroid = [
            (sum[0] / f64::from(count)) as f32,
            (sum[1] / f64::from(count)) as f32,
            (sum[2] / f64::from(count)) as f32,
        ];
        assert!(
            (0..3).any(|axis| wheel_min[axis] < body_min[axis] || wheel_max[axis] > body_max[axis]),
            "{} sits inside the body box {body_min:?}..{body_max:?}: \
             {wheel_min:?}..{wheel_max:?}",
            sub.name
        );
        centroids.push((sub.name.as_str(), centroid));
    }
    for (i, (_, a)) in centroids.iter().enumerate() {
        for (_, b) in &centroids[i + 1..] {
            let dist =
                ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
            assert!(
                dist > 0.1,
                "two wheel centroids {dist} m apart are the same wheel twice"
            );
        }
    }
    // The four centroids, recorded so an unexpected binary in the tree is
    // announced rather than discovered.
    for (name, centroid) in &centroids {
        println!("{name}: {centroid:?}");
    }
}

#[test]
fn the_fixtures_five_wheel_sub_meshes_are_not_one_sub_mesh() {
    // The anti-vacuity check: the five names are asserted in the task file, in
    // the fixture and above, so their agreement carries no information unless
    // something checks the fixture is **not** a single sub-mesh five times.
    let mesh = sedan();
    assert_eq!(mesh.sub_meshes.len(), 5);
    let wheels = &mesh.sub_meshes[1..];
    for (i, a) in wheels.iter().enumerate() {
        for b in &wheels[i + 1..] {
            let a_end = a.first_index + a.index_count;
            let b_end = b.first_index + b.index_count;
            assert!(
                a_end <= b.first_index || b_end <= a.first_index,
                "wheel ranges {} and {} overlap",
                a.name,
                b.name
            );
        }
        assert_ne!(
            mesh.sub_meshes[0].index_count, a.index_count,
            "the body holds as many triangles as a wheel"
        );
    }
}

#[test]
fn the_public_api_offers_the_byte_form_the_tests_use() {
    // The criterion's real content: the loader is reached through
    // `load_from_bytes` with a slice, so no filesystem stands between the
    // suite and the format. If a later refactor folded the byte form into
    // `load_from_path`, this fails to compile — which is the test.
    let bytes = include_bytes!("data/sedan.roados");
    let mesh = load_from_bytes(&bytes[..]).expect("the sedan fixture loads");
    assert_eq!(mesh.sub_meshes.len(), 5);
}
