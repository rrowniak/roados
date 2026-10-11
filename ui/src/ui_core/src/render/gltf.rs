//! glTF 2.0 / GLB model loading: the `gltf` crate, and the subset this demo
//! reads.
//!
//! A mesh that comes from **a file**, in the format the world already speaks:
//! [`load_from_bytes`] parses a binary glTF (GLB) through the [`gltf`] crate
//! and returns one [`GltfMesh`] per scene node that carries a mesh, each with
//! its geometry and **its node transform**. This replaces task 38's
//! project-owned model format and task 39's offline converter. Node placement
//! is a runtime concern again: the loader keeps the node transform and the demo
//! applies it at draw time through the `Mat4` that `DrawCommand::Mesh` already
//! carries (task 53).
//!
//! ## What is read
//!
//! `gltf::Gltf::from_slice` parses the GLB and keeps its BIN chunk;
//! [`gltf::Gltf::blob`] is the single embedded buffer. Each scene node with a
//! mesh becomes one [`GltfMesh`]: `node.transform()`, `node.mesh()`, and the
//! mesh's one primitive read through `primitive.reader(|buffer| …)` with
//! `read_positions`, `read_normals`, `read_tex_coords(0)` and `read_indices`
//! (`.into_u32()`). Vertices are built with [`MeshVertex::new`], so the file's
//! normals are renormalised here and a zero-length normal becomes
//! `[0.0, 0.0, 1.0]` rather than `NaN`. A `.gltf` (JSON)
//! container, an external or second buffer, a mesh with more than one
//! primitive, a primitive missing `POSITION`/`NORMAL`/`TEXCOORD_0`, a
//! non-`TRIANGLES` mode, a normalized or non-`f32` attribute, a node whose
//! transform is a matrix, a rotation or a non-unit scale, and an index that
//! escapes its vertex array are each [`GltfError::Unsupported`], naming what
//! was found. **This is the same subset the removed offline converter
//! enforced**, moved from conversion time to load time and no wider.
//!
//! ## Node transforms are translation-only, and that is deliberate
//!
//! The loader converts `node.transform()` to a [`Mat4`]: `Transform::Decomposed`
//! with an identity rotation and unit scale becomes
//! `Mat4::identity().translated(translation)`; anything else is `Unsupported`.
//! **This is correct and deliberate**: the renderer uploads `u_normal_matrix`
//! as the constant `IDENTITY_MAT3` (`render.rs`'s mesh draw pass), which is
//! only true while no node carries a rotation or a non-uniform scale, and
//! `Mat4` has no inverse-transpose to compute a real normal matrix from.
//! Supporting a rotating node is out of scope; the loader refuses it rather
//! than shading it wrong.
//!
//! ## The errors, and the repair each one asks for
//!
//! - [`GltfError::Io`]: the path could not be opened or read — fix the path.
//! - [`GltfError::Parse`]: the `gltf` crate could not parse the bytes — the
//!   parser stays the authority on JSON and GLB-frame faults.
//! - [`GltfError::Unsupported`]: valid glTF, outside the subset above — the
//!   message names the field, the mode, the transform or the index.
//!
//! ## Accepted limit: the crate's panics, not this loader's
//!
//! `gltf` 1.4.1 contains internal `unwrap`/`panic!` paths reachable from
//! malformed input; `MeshError` had none. The asset this demo loads is
//! committed and trusted, so the demo path does not exercise them, but
//! [`load_from_bytes`] is public and a caller may hand it untrusted bytes.
//! **This module records that as an accepted limit and does not claim
//! `MeshError`'s no-panic guarantee.**
//!
//! ## The `&Path` wrapper
//!
//! [`load_from_bytes`] is the whole of the loader and [`load_from_path`] is a
//! wrapper over it, because `AGENTS.md` permits no test that needs a filesystem
//! and every test in this module calls the byte form.

use std::path::Path;

use gltf::accessor::{DataType, Dimensions};
use gltf::buffer::Source;
use gltf::mesh::Mode;
use gltf::scene::Transform;
use gltf::Semantic;

use crate::render::matrix::Mat4;
use crate::render::mesh::{Mesh, MeshVertex, SubMesh};

/// The identity rotation a translation-only node carries, `[x, y, z, w]`.
///
/// Compared exactly: the `gltf` parser reads the file's own `f32`s, and a node
/// with no rotation is `[0.0, 0.0, 0.0, 1.0]` to the bit.
const IDENTITY_ROTATION: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// The unit scale a translation-only node carries, `[x, y, z]`.
const UNIT_SCALE: [f32; 3] = [1.0, 1.0, 1.0];

/// One glTF mesh instanced by one node: its geometry and its node transform.
///
/// `name` is the node's name, falling back to the mesh's, so a part keeps the
/// identity the scene gave it. `mesh` is task 35's [`Mesh`] with **one**
/// [`SubMesh`] covering all indices, because the glTF primitive's indices are
/// already absolute into its own vertex array. `transform` is the node's
/// placement, translation-only (see the module docs).
#[derive(Clone, Debug, PartialEq)]
pub struct GltfMesh {
    /// The instance's name, from the node (or the mesh when the node is
    /// unnamed).
    pub name: String,
    /// The geometry, with one sub-mesh covering all indices.
    pub mesh: Mesh,
    /// The node's placement: a translation-only matrix.
    pub transform: Mat4,
}

/// Why a GLB could not be loaded.
///
/// Three variants, in the crate's idiom — [`Clone`], [`Eq`], a hand-written
/// [`std::fmt::Display`] and a [`std::error::Error`] impl — so a caller can
/// hold one beside anything else an asset load returns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GltfError {
    /// The path could not be opened or read.
    ///
    /// Only [`load_from_path`] produces this: a missing file and a malformed
    /// file are different answers here.
    Io(String),
    /// The `gltf` crate could not parse the bytes. Carries the crate's own
    /// message, so the parser stays the authority on JSON and GLB faults.
    Parse(String),
    /// Valid glTF that is outside the subset this loader reads. Carries the
    /// field, mode, transform or index that was found.
    Unsupported(String),
}

impl std::fmt::Display for GltfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GltfError::Io(message) => write!(f, "could not read model file: {message}"),
            GltfError::Parse(message) => write!(f, "could not parse glTF: {message}"),
            GltfError::Unsupported(message) => write!(f, "unsupported glTF: {message}"),
        }
    }
}

impl std::error::Error for GltfError {}

/// Reads every mesh instance from a binary glTF (GLB) in memory.
///
/// One [`GltfMesh`] per scene node with a mesh, in node order, each keeping its
/// node transform. The subset and the refusals are the module docs'. A
/// `.gltf` (JSON) or any non-GLB input is [`GltfError::Unsupported`] before the
/// crate sees it, so the failure names what was handed in rather than
/// misreporting a JSON parse.
//
// (No `#[must_use]`: `Result` is already `#[must_use]`, and an explicit
// attribute on top of it is clippy's `double_must_use`.)
pub fn load_from_bytes(bytes: &[u8]) -> Result<Vec<GltfMesh>, GltfError> {
    if !bytes.starts_with(b"glTF") {
        return Err(GltfError::Unsupported(
            "not a binary glTF (GLB) container; a .gltf JSON file or another format is unsupported"
                .to_string(),
        ));
    }
    let gltf = gltf::Gltf::from_slice(bytes).map_err(map_parse_error)?;

    let buffer_count = gltf.document.buffers().count();
    if buffer_count != 1 {
        return Err(GltfError::Unsupported(format!(
            "expected a single embedded buffer, found {buffer_count}"
        )));
    }
    let Some(blob) = gltf.blob.as_deref() else {
        return Err(GltfError::Unsupported(
            "the glTF buffer is external (a URI); only a GLB with one embedded buffer is supported"
                .to_string(),
        ));
    };

    let mut loaded = Vec::new();
    for node in gltf.document.nodes() {
        let Some(part) = node.mesh() else {
            continue;
        };
        let name = node
            .name()
            .or_else(|| part.name())
            .unwrap_or("")
            .to_string();
        let transform = node_transform(&node)?;
        let mut primitives = part.primitives();
        let Some(primitive) = primitives.next() else {
            return Err(GltfError::Unsupported(format!(
                "mesh {name:?} has no primitives"
            )));
        };
        if primitives.next().is_some() {
            return Err(GltfError::Unsupported(format!(
                "mesh {name:?} has more than one primitive; this loader reads one"
            )));
        }
        let mesh = read_primitive(&primitive, blob, &name)?;
        loaded.push(GltfMesh {
            name,
            mesh,
            transform,
        });
    }
    Ok(loaded)
}

/// Loads every mesh instance from a GLB at a filesystem path.
///
/// A wrapper: it reads the whole file and calls [`load_from_bytes`], mapping an
/// `io::Error` to [`GltfError::Io`]. The buffer form is the one that exists;
/// this one is how the demo reaches it. (No `#[must_use]`: `Result`.)
pub fn load_from_path(path: &Path) -> Result<Vec<GltfMesh>, GltfError> {
    let bytes = std::fs::read(path)
        .map_err(|error| GltfError::Io(format!("{}: {error}", path.display())))?;
    load_from_bytes(&bytes)
}

/// Maps the crate's own error into this loader's taxonomy.
///
/// A **validation** fault is the crate's report that the document is not the
/// glTF subset this loader reads — a missing `POSITION`, a bad accessor shape,
/// a dangling buffer view — so it is [`GltfError::Unsupported`], and its
/// message (the crate's, which names the offending path) is what names the
/// field. Every other fault is a parse or GLB-frame fault and stays
/// [`GltfError::Parse`].
fn map_parse_error(error: gltf::Error) -> GltfError {
    match error {
        gltf::Error::Validation(_) => GltfError::Unsupported(format!(
            "the GLB does not satisfy the supported subset: {error}"
        )),
        other => GltfError::Parse(other.to_string()),
    }
}

/// Converts a node's transform to a translation-only [`Mat4`], or refuses it.
///
/// See the module docs: a rotation or non-unit scale would need a real normal
/// matrix, which this renderer does not have, so it is refused by name rather
/// than shaded wrong. A `Transform::Matrix` is refused for the same reason —
/// there is no way to know from the matrix alone that it is translation-only
/// without decomposing it, and the decomposed form is what the file could have
/// carried.
fn node_transform(node: &gltf::Node<'_>) -> Result<Mat4, GltfError> {
    match node.transform() {
        Transform::Decomposed {
            translation,
            rotation,
            scale,
        } => {
            if rotation != IDENTITY_ROTATION {
                return Err(GltfError::Unsupported(format!(
                    "node transform has rotation {rotation:?}; only a translation is supported"
                )));
            }
            if scale != UNIT_SCALE {
                return Err(GltfError::Unsupported(format!(
                    "node transform has scale {scale:?}; only a unit scale is supported"
                )));
            }
            Ok(Mat4::identity().translated(translation))
        }
        Transform::Matrix { matrix } => Err(GltfError::Unsupported(format!(
            "node transform is a 4x4 matrix {matrix:?}; only a translation is supported"
        ))),
    }
}

/// Reads one primitive's geometry into task 35's [`Mesh`].
///
/// Checks the mode and the four attributes' types before reading, so a
/// wrongly-typed accessor is refused rather than `debug_assert`ed in the
/// crate's iterator, then reads positions, normals, UVs and indices, builds
/// every vertex through [`MeshVertex::new`] and refuses any index past the
/// vertex array.
fn read_primitive(
    primitive: &gltf::Primitive<'_>,
    blob: &[u8],
    name: &str,
) -> Result<Mesh, GltfError> {
    if primitive.mode() != Mode::Triangles {
        return Err(GltfError::Unsupported(format!(
            "mesh {name:?} has mode {:?}; only TRIANGLES is supported",
            primitive.mode()
        )));
    }
    check_attribute(
        primitive,
        &Semantic::Positions,
        DataType::F32,
        Dimensions::Vec3,
        name,
        "POSITION",
    )?;
    check_attribute(
        primitive,
        &Semantic::Normals,
        DataType::F32,
        Dimensions::Vec3,
        name,
        "NORMAL",
    )?;
    check_attribute(
        primitive,
        &Semantic::TexCoords(0),
        DataType::F32,
        Dimensions::Vec2,
        name,
        "TEXCOORD_0",
    )?;
    let Some(indices_accessor) = primitive.indices() else {
        return Err(GltfError::Unsupported(format!(
            "mesh {name:?} has no index accessor; only indexed geometry is supported"
        )));
    };
    check_accessor(
        &indices_accessor,
        name,
        "indices",
        &[DataType::U8, DataType::U16, DataType::U32],
        Dimensions::Scalar,
    )?;

    let reader = primitive.reader(|buffer| match buffer.source() {
        Source::Bin => Some(blob),
        Source::Uri(_) => None,
    });
    let positions = reader.read_positions().ok_or_else(|| {
        GltfError::Unsupported(format!("mesh {name:?} POSITION data could not be read"))
    })?;
    let positions: Vec<[f32; 3]> = positions.collect();
    let normals = reader.read_normals().ok_or_else(|| {
        GltfError::Unsupported(format!("mesh {name:?} NORMAL data could not be read"))
    })?;
    let normals: Vec<[f32; 3]> = normals.collect();
    let uvs = reader.read_tex_coords(0).ok_or_else(|| {
        GltfError::Unsupported(format!("mesh {name:?} TEXCOORD_0 data could not be read"))
    })?;
    let uvs: Vec<[f32; 2]> = uvs.into_f32().collect();
    if positions.len() != normals.len() || positions.len() != uvs.len() {
        return Err(GltfError::Unsupported(format!(
            "mesh {name:?} attribute counts differ: {} positions, {} normals, {} uvs",
            positions.len(),
            normals.len(),
            uvs.len()
        )));
    }
    let vertices: Vec<MeshVertex> = positions
        .iter()
        .zip(normals.iter())
        .zip(uvs.iter())
        .map(|((position, normal), uv)| MeshVertex::new(*position, *normal, *uv))
        .collect();

    let indices = reader.read_indices().ok_or_else(|| {
        GltfError::Unsupported(format!("mesh {name:?} index data could not be read"))
    })?;
    let indices: Vec<u32> = indices.into_u32().collect();
    let vertex_count = u32::try_from(vertices.len()).map_err(|_| {
        GltfError::Unsupported(format!(
            "mesh {name:?} has more vertices than a u32 can count"
        ))
    })?;
    for (position, index) in indices.iter().enumerate() {
        if *index >= vertex_count {
            return Err(GltfError::Unsupported(format!(
                "mesh {name:?} index {index} at position {position} reaches past its \
                 {vertex_count} vertices"
            )));
        }
    }
    let index_count = u32::try_from(indices.len()).map_err(|_| {
        GltfError::Unsupported(format!(
            "mesh {name:?} has more indices than a u32 can count"
        ))
    })?;
    let sub_mesh = SubMesh {
        name: name.to_string(),
        first_index: 0,
        index_count,
    };
    Ok(Mesh {
        vertices,
        indices,
        sub_meshes: vec![sub_mesh],
    })
}

/// Checks that `semantic` is present and has the expected type.
fn check_attribute(
    primitive: &gltf::Primitive<'_>,
    semantic: &Semantic,
    data_type: DataType,
    dimensions: Dimensions,
    mesh_name: &str,
    label: &str,
) -> Result<(), GltfError> {
    let Some(accessor) = primitive.get(semantic) else {
        return Err(GltfError::Unsupported(format!(
            "mesh {mesh_name:?} has no {label} attribute"
        )));
    };
    check_accessor(&accessor, mesh_name, label, &[data_type], dimensions)
}

/// Checks an accessor is non-normalized and of one of `types` and `dimensions`.
fn check_accessor(
    accessor: &gltf::Accessor<'_>,
    mesh_name: &str,
    label: &str,
    types: &[DataType],
    dimensions: Dimensions,
) -> Result<(), GltfError> {
    if accessor.normalized() {
        return Err(GltfError::Unsupported(format!(
            "mesh {mesh_name:?} {label} accessor is normalized; only non-normalized attributes \
             are supported"
        )));
    }
    if accessor.dimensions() != dimensions || !types.contains(&accessor.data_type()) {
        return Err(GltfError::Unsupported(format!(
            "mesh {mesh_name:?} {label} accessor is {:?} {:?}; expected {dimensions:?} of {types:?}",
            accessor.data_type(),
            accessor.dimensions()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The accessors of the minimal fixture: one `VEC3` `f32` position (with
    /// the `min`/`max` glTF requires), normal and UV, and one `SCALAR` `u16`
    /// index array.
    const ACCESSORS: &str = concat!(
        r#"[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","#,
        r#""min":[0.0,0.0,0.0],"max":[1.0,1.0,0.0]},"#,
        r#"{"bufferView":1,"componentType":5126,"count":3,"type":"VEC3"},"#,
        r#"{"bufferView":2,"componentType":5126,"count":3,"type":"VEC2"},"#,
        r#"{"bufferView":3,"componentType":5123,"count":3,"type":"SCALAR"}]"#
    );

    /// The four buffer views of the minimal fixture, laid out back to back.
    const VIEWS: &str = concat!(
        r#"[{"buffer":0,"byteOffset":0,"byteLength":36},"#,
        r#"{"buffer":0,"byteOffset":36,"byteLength":36},"#,
        r#"{"buffer":0,"byteOffset":72,"byteLength":24},"#,
        r#"{"buffer":0,"byteOffset":96,"byteLength":6}]"#
    );

    /// Two nodes, both instancing the one mesh, each with its own translation.
    const NODES: &str = concat!(
        r#"[{"mesh":0,"translation":[1.0,2.0,3.0],"name":"first"},"#,
        r#"{"mesh":0,"translation":[4.0,5.0,6.0],"name":"second"}]"#
    );

    /// One mesh, one `TRIANGLES` primitive over the four accessors.
    const MESH_ONE: &str = concat!(
        r#"[{"name":"tri","primitives":[{"#,
        r#""attributes":{"POSITION":0,"NORMAL":1,"TEXCOORD_0":2},"indices":3,"mode":4}]}]"#
    );

    /// One vertex per position, normal and UV, then three `u16` indices:
    /// positions, normals, UVs and indices back to back, 102 bytes.
    fn triangle_bin(indices: [u16; 3]) -> Vec<u8> {
        let positions = [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = [[0.0f32, 0.0, 1.0]; 3];
        let uvs = [[0.0f32, 0.0], [1.0, 0.0], [0.0, 1.0]];
        let mut out = Vec::new();
        for position in &positions {
            for value in position {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        for normal in &normals {
            for value in normal {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        for uv in &uvs {
            for value in uv {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        for index in indices {
            out.extend_from_slice(&index.to_le_bytes());
        }
        out
    }

    /// Wraps a JSON document and binary payload in the GLB container: the
    /// twelve-byte header, a padded `JSON` chunk and a padded `BIN` chunk.
    fn glb(document: &str, bin: &[u8]) -> Vec<u8> {
        let mut json = document.as_bytes().to_vec();
        while json.len() % 4 != 0 {
            json.push(b' ');
        }
        let mut bin = bin.to_vec();
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let total = 12 + 8 + json.len() + 8 + bin.len();
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(b"glTF");
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&u32::try_from(total).unwrap().to_le_bytes());
        out.extend_from_slice(&u32::try_from(json.len()).unwrap().to_le_bytes());
        out.extend_from_slice(b"JSON");
        out.extend_from_slice(&json);
        out.extend_from_slice(&u32::try_from(bin.len()).unwrap().to_le_bytes());
        out.extend_from_slice(b"BIN\0");
        out.extend_from_slice(&bin);
        out
    }

    /// A document with the given mesh, nodes and accessors and the fixture's
    /// fixed buffer views and buffer.
    fn document(meshes: &str, nodes: &str, accessors: &str) -> String {
        format!(
            concat!(
                r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0,1]}}],"#,
                r#""nodes":{nodes},"meshes":{meshes},"accessors":{accessors},"#,
                r#""bufferViews":{views},"buffers":[{{"byteLength":102}}]}}"#
            ),
            nodes = nodes,
            meshes = meshes,
            accessors = accessors,
            views = VIEWS
        )
    }

    /// The two-node fixture: one mesh instanced by two nodes, each translated.
    fn two_node_glb() -> Vec<u8> {
        glb(
            &document(MESH_ONE, NODES, ACCESSORS),
            &triangle_bin([0, 1, 2]),
        )
    }

    #[test]
    fn the_magic_and_a_two_node_fixture_parse_to_two_instances() {
        let meshes = load_from_bytes(&two_node_glb()).unwrap();
        assert_eq!(meshes.len(), 2, "one instance per node with a mesh");
        assert_eq!(meshes[0].name, "first");
        assert_eq!(meshes[1].name, "second");
        for mesh in &meshes {
            assert_eq!(mesh.mesh.vertices.len(), 3);
            assert_eq!(mesh.mesh.indices, vec![0, 1, 2]);
            assert_eq!(mesh.mesh.sub_meshes.len(), 1, "one sub-mesh covers all");
            assert_eq!(mesh.mesh.sub_meshes[0].first_index, 0);
            assert_eq!(mesh.mesh.sub_meshes[0].index_count, 3);
        }
    }

    #[test]
    fn each_nodes_translation_becomes_its_own_matrix() {
        // The load-bearing conversion: node 0 is placed at (1,2,3), node 1 at
        // (4,5,6), and the origin maps through each. A loader that returned
        // `Mat4::identity()` for every node sends both to the origin and this
        // is the assertion that sees it.
        let meshes = load_from_bytes(&two_node_glb()).unwrap();
        assert_eq!(
            meshes[0].transform,
            Mat4::identity().translated([1.0, 2.0, 3.0])
        );
        assert_eq!(
            meshes[1].transform,
            Mat4::identity().translated([4.0, 5.0, 6.0])
        );
        assert_eq!(
            meshes[0].transform.transform_point([0.0, 0.0, 0.0]),
            Some([1.0, 2.0, 3.0])
        );
        assert_ne!(
            meshes[0].transform, meshes[1].transform,
            "the two placements differ"
        );
    }

    #[test]
    fn the_fixtures_normals_are_renormalised_and_a_zero_normal_becomes_up() {
        // `MeshVertex::new` is what every vertex is built through, so a
        // zero-length normal becomes up rather than `NaN`. The fixture's bin
        // is rewritten with a zero normal for the third vertex.
        let mut bin = triangle_bin([0, 1, 2]);
        for byte in bin[36 + 2 * 12..36 + 3 * 12].iter_mut() {
            *byte = 0;
        }
        let bytes = glb(&document(MESH_ONE, NODES, ACCESSORS), &bin);
        let meshes = load_from_bytes(&bytes).unwrap();
        assert_eq!(meshes[0].mesh.vertices[0].normal, [0.0, 0.0, 1.0]);
        assert_eq!(meshes[0].mesh.vertices[2].normal, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn a_json_gltf_document_is_unsupported_and_is_not_parsed_as_json() {
        // A `.gltf` is JSON and would deserialize in the crate; the magic check
        // refuses it first, so the message names the container rather than
        // reporting a JSON error at some interior byte.
        let error = load_from_bytes(br#"{"asset":{"version":"2.0"}}"#).unwrap_err();
        assert!(
            matches!(&error, GltfError::Unsupported(message) if message.contains("binary glTF")),
            "expected an Unsupported naming the container, got {error:?}"
        );
    }

    #[test]
    fn a_mesh_with_no_position_is_unsupported() {
        // The crate's validator requires `POSITION` (and its `min`/`max`), so
        // this is refused as a validation fault, which `map_parse_error` turns
        // into `Unsupported` naming the field.
        let mesh = concat!(
            r#"[{"name":"tri","primitives":[{"#,
            r#""attributes":{"NORMAL":1,"TEXCOORD_0":2},"indices":3,"mode":4}]}]"#
        );
        let error = load_from_bytes(&glb(
            &document(mesh, NODES, ACCESSORS),
            &triangle_bin([0, 1, 2]),
        ))
        .unwrap_err();
        assert!(
            matches!(&error, GltfError::Unsupported(message) if message.contains("POSITION")),
            "expected an Unsupported naming POSITION, got {error:?}"
        );
    }

    #[test]
    fn a_mesh_with_no_normal_is_unsupported() {
        let mesh = concat!(
            r#"[{"name":"tri","primitives":[{"#,
            r#""attributes":{"POSITION":0,"TEXCOORD_0":2},"indices":3,"mode":4}]}]"#
        );
        let error = load_from_bytes(&glb(
            &document(mesh, NODES, ACCESSORS),
            &triangle_bin([0, 1, 2]),
        ))
        .unwrap_err();
        assert!(
            matches!(&error, GltfError::Unsupported(message) if message.contains("NORMAL")),
            "expected an Unsupported naming NORMAL, got {error:?}"
        );
    }

    #[test]
    fn a_mesh_with_no_texcoord_is_unsupported() {
        let mesh = concat!(
            r#"[{"name":"tri","primitives":[{"#,
            r#""attributes":{"POSITION":0,"NORMAL":1},"indices":3,"mode":4}]}]"#
        );
        let error = load_from_bytes(&glb(
            &document(mesh, NODES, ACCESSORS),
            &triangle_bin([0, 1, 2]),
        ))
        .unwrap_err();
        assert!(
            matches!(&error, GltfError::Unsupported(message) if message.contains("TEXCOORD_0")),
            "expected an Unsupported naming TEXCOORD_0, got {error:?}"
        );
    }

    #[test]
    fn a_non_triangle_mode_is_unsupported() {
        let mesh = concat!(
            r#"[{"name":"tri","primitives":[{"#,
            r#""attributes":{"POSITION":0,"NORMAL":1,"TEXCOORD_0":2},"indices":3,"mode":1}]}]"#
        );
        let error = load_from_bytes(&glb(
            &document(mesh, NODES, ACCESSORS),
            &triangle_bin([0, 1, 2]),
        ))
        .unwrap_err();
        assert!(
            matches!(&error, GltfError::Unsupported(message) if message.contains("mode")),
            "expected an Unsupported naming the mode, got {error:?}"
        );
    }

    #[test]
    fn a_mesh_with_two_primitives_is_unsupported() {
        let mesh = concat!(
            r#"[{"name":"tri","primitives":["#,
            r#"{"attributes":{"POSITION":0,"NORMAL":1,"TEXCOORD_0":2},"indices":3,"mode":4},"#,
            r#"{"attributes":{"POSITION":0,"NORMAL":1,"TEXCOORD_0":2},"indices":3,"mode":4}]}]"#
        );
        let error = load_from_bytes(&glb(
            &document(mesh, NODES, ACCESSORS),
            &triangle_bin([0, 1, 2]),
        ))
        .unwrap_err();
        assert!(
            matches!(&error, GltfError::Unsupported(message) if message.contains("primitive")),
            "expected an Unsupported naming the primitive, got {error:?}"
        );
    }

    #[test]
    fn a_normalized_attribute_is_unsupported() {
        // `TEXCOORD_0` as a normalized `u16`: valid glTF, outside the subset.
        let accessors = ACCESSORS.replace(
            r#"{"bufferView":2,"componentType":5126,"count":3,"type":"VEC2"}"#,
            r#"{"bufferView":2,"componentType":5123,"count":3,"type":"VEC2","normalized":true}"#,
        );
        let error = load_from_bytes(&glb(
            &document(MESH_ONE, NODES, &accessors),
            &triangle_bin([0, 1, 2]),
        ))
        .unwrap_err();
        assert!(
            matches!(&error, GltfError::Unsupported(message) if message.contains("normalized")),
            "expected an Unsupported naming normalization, got {error:?}"
        );
    }

    #[test]
    fn a_second_buffer_is_unsupported() {
        let document = document(MESH_ONE, NODES, ACCESSORS).replace(
            r#""buffers":[{"byteLength":102}]"#,
            r#""buffers":[{"byteLength":102},{"byteLength":8}]"#,
        );
        let error = load_from_bytes(&glb(&document, &triangle_bin([0, 1, 2]))).unwrap_err();
        assert!(
            matches!(&error, GltfError::Unsupported(message) if message.contains("single embedded buffer")),
            "expected an Unsupported naming the buffer count, got {error:?}"
        );
    }

    #[test]
    fn an_index_past_the_vertex_array_is_unsupported() {
        let error = load_from_bytes(&glb(
            &document(MESH_ONE, NODES, ACCESSORS),
            &triangle_bin([0, 1, 9]),
        ))
        .unwrap_err();
        assert!(
            matches!(&error, GltfError::Unsupported(message) if message.contains("index 9")),
            "expected an Unsupported naming the index, got {error:?}"
        );
    }

    #[test]
    fn a_rotated_node_is_unsupported() {
        let nodes = r#"[{"mesh":0,"rotation":[0.0,0.0,0.0,0.9],"name":"spin"},{"mesh":0}]"#;
        let error = load_from_bytes(&glb(
            &document(MESH_ONE, nodes, ACCESSORS),
            &triangle_bin([0, 1, 2]),
        ))
        .unwrap_err();
        assert!(
            matches!(&error, GltfError::Unsupported(message) if message.contains("rotation")),
            "expected an Unsupported naming the rotation, got {error:?}"
        );
    }

    #[test]
    fn a_scaled_node_is_unsupported() {
        let nodes = r#"[{"mesh":0,"scale":[2.0,2.0,2.0],"name":"big"},{"mesh":0}]"#;
        let error = load_from_bytes(&glb(
            &document(MESH_ONE, nodes, ACCESSORS),
            &triangle_bin([0, 1, 2]),
        ))
        .unwrap_err();
        assert!(
            matches!(&error, GltfError::Unsupported(message) if message.contains("scale")),
            "expected an Unsupported naming the scale, got {error:?}"
        );
    }

    #[test]
    fn a_matrix_node_is_unsupported() {
        let nodes = concat!(
            r#"[{"mesh":0,"name":"matrix","matrix":"#,
            r#"[1.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0,0.0,0.0,1.0,0.0,1.0,2.0,3.0,1.0]},"#,
            r#"{"mesh":0}]"#
        );
        let error = load_from_bytes(&glb(
            &document(MESH_ONE, nodes, ACCESSORS),
            &triangle_bin([0, 1, 2]),
        ))
        .unwrap_err();
        assert!(
            matches!(&error, GltfError::Unsupported(message) if message.contains("matrix")),
            "expected an Unsupported naming the matrix, got {error:?}"
        );
    }
}
