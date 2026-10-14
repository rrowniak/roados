//! The committed GLB's shape, exercised without a filesystem.
//!
//! `AGENTS.md` permits no test that needs a filesystem, and the resolution is
//! the API shape: [`load_from_bytes`](ui_core::render::gltf::load_from_bytes)
//! is the whole of the loader and `load_from_path` is a wrapper over it, so
//! this test holds the committed asset in memory via `include_bytes!`.
//!
//! **It includes the committed asset, not a second fixture.** A duplicate
//! binary under `tests/data/` would be a second thing to drift; the file the
//! demo draws is the file this test reads. The assertions are task 39's
//! fixture's, now against real GLB bytes and — the load-bearing change — against
//! the loader's **runtime node transforms**: with geometry authored at the
//! origin, a loader that dropped or mis-converted a node transform puts all
//! four wheels at the origin inside the body, and the distinct-placements
//! assertion below is what sees it.

use ui_core::render::gltf::{load_from_bytes, GltfMesh};
use ui_core::render::mesh::Mesh;

/// The committed asset, held in memory: no filesystem, no wall clock.
fn sedan() -> Vec<GltfMesh> {
    let bytes = include_bytes!("../../ui_demo/assets/sedan.glb");
    load_from_bytes(&bytes[..]).expect("the sedan GLB loads")
}

/// A part's world-space bounding box and index-weighted centroid, in metres.
fn transformed_bounds(part: &GltfMesh) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    let mut sum = [0.0f64; 3];
    let mut count = 0u32;
    for index in &part.mesh.indices {
        let local = part.mesh.vertices[*index as usize].position;
        let world = part
            .transform
            .transform_point(local)
            .expect("a translation never produces a w of zero");
        for axis in 0..3 {
            min[axis] = min[axis].min(world[axis]);
            max[axis] = max[axis].max(world[axis]);
            sum[axis] += f64::from(world[axis]);
        }
        count += 1;
    }
    let centroid = [
        (sum[0] / f64::from(count)) as f32,
        (sum[1] / f64::from(count)) as f32,
        (sum[2] / f64::from(count)) as f32,
    ];
    (min, max, centroid)
}

#[test]
fn the_committed_sedan_loads_as_five_named_parts_with_the_verified_counts() {
    let parts = sedan();
    assert_eq!(parts.len(), 5, "five nodes carry a mesh");

    let names: Vec<&str> = parts.iter().map(|part| part.name.as_str()).collect();
    for expected in [
        "body",
        "wheel-front-left",
        "wheel-front-right",
        "wheel-back-left",
        "wheel-back-right",
    ] {
        assert!(
            names.contains(&expected),
            "the {expected} part is present: {names:?}"
        );
    }

    let triangles = |part: &GltfMesh| part.mesh.indices.len() / 3;
    let total: usize = parts.iter().map(triangles).sum();
    assert_eq!(total, 2032, "704 body triangles plus four wheels of 332");
    for part in &parts {
        let want = if part.name == "body" { 704 } else { 332 };
        assert_eq!(triangles(part), want, "{} triangle count", part.name);
    }
}

#[test]
fn every_part_is_internally_valid() {
    for part in sedan() {
        let vertices = part.mesh.vertices.len();
        assert_eq!(part.mesh.sub_meshes.len(), 1, "{}: one sub-mesh", part.name);
        assert_eq!(
            part.mesh.sub_meshes[0].index_count as usize,
            part.mesh.indices.len(),
            "{}: the sub-mesh covers every index",
            part.name
        );
        for (position, index) in part.mesh.indices.iter().enumerate() {
            assert!(
                (*index as usize) < vertices,
                "{}: index {index} at position {position} past {vertices} vertices",
                part.name
            );
        }
        for vertex in &part.mesh.vertices {
            let length = (vertex.normal[0] * vertex.normal[0]
                + vertex.normal[1] * vertex.normal[1]
                + vertex.normal[2] * vertex.normal[2])
                .sqrt();
            assert!(
                (length - 1.0).abs() < 1e-5,
                "{}: a normal of length {length} is not a direction",
                part.name
            );
        }
    }
}

#[test]
fn the_node_transforms_place_the_four_wheels_outside_the_body_and_apart() {
    // The load-bearing one. The body's box is its own transformed geometry's
    // bbox; every wheel's transformed geometry must reach outside it in at
    // least one axis (the wheels touch the ground at y = 0 while the body sits
    // at y = 0.15), and the four wheel centroids must be more than 0.1 m apart.
    // Identity-ing the node transforms puts all four wheels at the origin on
    // top of each other, and the distinctness assertion fails for the right
    // reason.
    let parts = sedan();
    let body = parts
        .iter()
        .find(|part| part.name == "body")
        .expect("the sedan has a body");
    let (body_min, body_max, _) = transformed_bounds(body);

    let mut centroids = Vec::new();
    for part in parts.iter().filter(|part| part.name != "body") {
        let (wheel_min, wheel_max, centroid) = transformed_bounds(part);
        assert!(
            (0..3).any(|axis| wheel_min[axis] < body_min[axis] || wheel_max[axis] > body_max[axis]),
            "{} sits inside the body box {body_min:?}..{body_max:?}: {wheel_min:?}..{wheel_max:?}",
            part.name
        );
        centroids.push((part.name.as_str(), centroid));
    }
    assert_eq!(centroids.len(), 4);
    for (i, (name_a, a)) in centroids.iter().enumerate() {
        for (name_b, b) in &centroids[i + 1..] {
            let distance =
                ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
            assert!(
                distance > 0.1,
                "{name_a} and {name_b} centroids are {distance} m apart: the same wheel twice"
            );
        }
    }
    // The four centroids, recorded so an unexpected binary in the tree is
    // announced rather than discovered.
    for (name, centroid) in &centroids {
        println!("{name}: {centroid:?}");
    }
    // And the body's own placement is the GLB's `body` node translation, which
    // pins the conversion on a part whose translation is not mirrored.
    assert_eq!(
        body.transform,
        ui_core::render::matrix::Mat4::identity().translated([0.0, 0.15, -0.025])
    );
}

#[test]
fn the_loader_reports_the_sedans_minimum_length_byte_form() {
    // The public API's byte form is what this test uses and what the loader's
    // unit tests use; if a later refactor folded it into `load_from_path`, this
    // fails to compile, which is the test.
    let bytes = include_bytes!("../../ui_demo/assets/sedan.glb");
    let mesh: Vec<GltfMesh> = load_from_bytes(&bytes[..]).expect("the sedan GLB loads");
    let total: usize = mesh.iter().map(|part| part.mesh.indices.len() / 3).sum();
    assert_eq!(total, 2032);
    let _: Mesh = mesh.into_iter().next().expect("at least one part").mesh;
}

/// The sedan loads byte-identically after the hierarchy widening.
///
/// One FNV-1a hash over the whole load — names, transforms, vertices,
/// indices in order — pinned to the value the translation-only loader
/// produced. Translation-only worlds keep the old representation (local
/// vertices under a translation matrix), so any byte that moved fails here
/// rather than in a rendering diff. Baked (rotated/scaled/matrix) worlds are
/// covered by the loader's own unit tests.
#[test]
fn the_sedan_loads_byte_identically() {
    let parts = sedan();
    let mut hash: u64 = 14695981039346656037;
    let mut mix = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(1099511628211);
        }
    };
    for part in &parts {
        mix(part.name.as_bytes());
        for value in part.transform.as_slice() {
            mix(&value.to_le_bytes());
        }
        for vertex in &part.mesh.vertices {
            for value in vertex
                .position
                .iter()
                .chain(vertex.normal.iter())
                .chain(vertex.uv.iter())
            {
                mix(&value.to_le_bytes());
            }
        }
        for index in &part.mesh.indices {
            mix(&index.to_le_bytes());
        }
    }
    assert_eq!(
        hash, 502188356119884179,
        "the sedan load moved; the translation fast path must stay byte-identical"
    );
}

/// The sedan's one material comes through on every part: the colormap file
/// beside the model, opaque white factor (the document sets none).
#[test]
fn the_sedan_parts_carry_the_colormap_material() {
    for part in sedan() {
        assert_eq!(
            part.material.base_color_texture.as_deref(),
            Some("colormap.png"),
            "{}: texture file name",
            part.name
        );
        assert_eq!(
            part.material.base_color_factor,
            [1.0, 1.0, 1.0, 1.0],
            "{}: default factor",
            part.name
        );
    }
}
