//! glTF 2.0 / GLB model loading: the `gltf` crate, and the subset this demo
//! reads.
//!
//! A mesh that comes from **a file**, in the format the world already speaks:
//! [`load_from_bytes`] parses a glTF 2.0 document — a binary GLB or a `.gltf`
//! JSON document — through the [`gltf`] crate and returns one [`GltfMesh`] per
//! scene node that carries a mesh, each with its geometry baked into world
//! space and its base-color material. This replaces task 38's project-owned
//! model format and task 39's offline converter. Node placement is resolved
//! at load time again: the loader walks the default scene's hierarchy,
//! accumulates each node's world matrix and bakes it into the vertices, so
//! the demo draws every part with the identity placement the loader returns
//! (task 53 and the Passat widening).
//!
//! ## What is read
//!
//! `gltf::Gltf::from_slice` parses either container — a GLB, keeping its BIN
//! chunk as [`gltf::Gltf::blob`], or a `.gltf` JSON document — with no new
//! dependency and no feature change (`default-features = false`,
//! `features = ["utils", "names"]` stays as task 53 set it; `import` stays
//! off, so no `base64`, no `image`, no `urlencoding`). Each scene node with a
//! mesh becomes one [`GltfMesh`]: the node's world matrix
//! (`parent * local`, down from the default scene's roots, from both the
//! `Decomposed`-with-quaternion and the `Matrix` local forms through the
//! crate's `Transform::matrix`), the mesh's one primitive read through
//! `primitive.reader(|buffer| …)` with `read_positions`, `read_normals`,
//! `read_tex_coords(0)` and `read_indices` (`.into_u32()`), and the
//! primitive's material as [`GltfMaterial`]. Vertices are built with
//! [`MeshVertex::new`], so the file's normals are renormalised here and a
//! zero-length normal becomes `[0.0, 0.0, 1.0]` rather than `NaN`. A mesh
//! with more than one primitive, a primitive missing
//! `POSITION`/`NORMAL`/`TEXCOORD_0`, a non-`TRIANGLES` mode, a normalized or
//! non-`f32` attribute, a sparse accessor, a Draco-compressed primitive and an
//! index that escapes its vertex array are each [`GltfError::Unsupported`],
//! naming what was found. **The translation-only refusals task 53 recorded
//! are gone**: a rotation, a scale or a 4x4 matrix node now loads, baked.
//!
//! ## Node transforms are baked; the placement left behind is trivial
//!
//! A world matrix that is exactly a translation keeps the old representation:
//! local vertices and `Mat4::identity().translated(offset)`, bit-identical
//! with what task 53's loader returned, so the committed Kenney sedan loads
//! byte-for-byte as before. Any other world — a rotation, a scale, a 4x4
//! matrix, or a translation reached through one — is baked into the vertices:
//! positions through the full 4x4, normals through its inverse-transpose
//! 3x3 ([`Mat4::transform_normal`], renormalised by [`MeshVertex::new`]), and
//! the part is emitted with [`Mat4::identity`]. Either way the renderer sees
//! a translation at most, so `render.rs`'s mesh draw pass keeps uploading
//! `u_normal_matrix` as the constant `IDENTITY_MAT3` with no new uniform and
//! no renderer change.
//!
//! ## The errors, and the repair each one asks for
//!
//! - [`GltfError::Io`]: the path could not be opened or read, or a sibling
//!   `.bin` beside a `.gltf` could not be — fix the path.
//! - [`GltfError::Parse`]: the `gltf` crate could not parse the bytes — the
//!   parser stays the authority on JSON and GLB-frame faults.
//! - [`GltfError::Unsupported`]: valid glTF, outside the subset above — the
//!   message names the field, the mode, the buffer or the index. A data-URI
//!   buffer and an embedded base64 image land here too: decoding either needs
//!   the `import` feature, which stays off by the dependency decision above.
//!
//! ## Accepted limit: the crate's panics, not this loader's
//!
//! `gltf` 1.4.1 contains internal `unwrap`/`panic!` paths reachable from
//! malformed input; `MeshError` had none. The assets this demo loads are
//! committed and trusted, so the demo path does not exercise them, but
//! [`load_from_bytes`] is public and a caller may hand it untrusted bytes.
//! **This module records that as an accepted limit and does not claim
//! `MeshError`'s no-panic guarantee.**
//!
//! ## The `&Path` wrapper
//!
//! [`load_from_bytes`] loads a GLB, and a `.gltf` whose buffers need no
//! sibling files; [`load_from_path`] additionally reads a `.gltf`'s sibling
//! `.bin` files relative to it, because `AGENTS.md` permits no test that
//! needs a filesystem and every buffer test calls the slice core instead.

use std::path::Path;

use gltf::accessor::{DataType, Dimensions};
use gltf::buffer::Source;
use gltf::mesh::Mode;
use gltf::Semantic;

use crate::render::matrix::Mat4;
use crate::render::mesh::{Mesh, MeshVertex, SubMesh};

/// One primitive's base-color material: its name, the texture file and the
/// factor tint.
///
/// `name` is the material's name (`"Paint"`), empty when the document names
/// none, so a caller can pick parts by material. `base_color_texture` is the image file's name (`"colormap.png"`, not the
/// `"Textures/colormap.png"` URI the document carries), because the demo
/// resolves it beside the model file; `None` when the primitive has no
/// texture — no material at all, an embedded buffer-view image, or a data
/// URI, none of which names a sibling file. `base_color_factor` is the
/// document's rgba factor, opaque white when the document sets none, and the
/// demo multiplies it as tint and opacity. Every other material property —
/// metallic and roughness factors, normal, occlusion and emissive textures —
/// is ignored: geometry plus base color is the subset, here as the type.
#[derive(Clone, Debug, PartialEq)]
pub struct GltfMaterial {
    /// The material's name, empty when the document names none.
    pub name: String,
    /// The texture image's file name, for the demo to resolve beside the
    /// model, or `None` when no sibling file is named.
    pub base_color_texture: Option<String>,
    /// The rgba base-color factor, opaque white by default.
    pub base_color_factor: [f32; 4],
}

/// One glTF mesh instanced by one node: its geometry and its material.
///
/// `name` is the node's name, falling back to the mesh's, so a part keeps the
/// identity the scene gave it. `mesh` is task 35's [`Mesh`] with **one**
/// [`SubMesh`] covering all indices, because the glTF primitive's indices are
/// already absolute into its own vertex array. `transform` is the node's
/// world placement, which is a translation at most: a translation-only world
/// stays a translation matrix over local vertices, anything else is baked
/// into the vertices (see the module docs) and this is
/// [`Mat4::identity`]. `material` is the primitive's base color.
#[derive(Clone, Debug, PartialEq)]
pub struct GltfMesh {
    /// The instance's name, from the node (or the mesh when the node is
    /// unnamed).
    pub name: String,
    /// The geometry, with one sub-mesh covering all indices.
    pub mesh: Mesh,
    /// The node's world placement: a translation, or identity when baked.
    pub transform: Mat4,
    /// The primitive's base-color material.
    pub material: GltfMaterial,
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

/// Reads every mesh instance from a glTF document in memory.
///
/// A binary GLB or a `.gltf` JSON document: [`gltf::Gltf::from_slice`]
/// accepts either with no new dependency. One [`GltfMesh`] per scene node
/// with a mesh, in node order, each with its world baked as the module docs
/// describe. A `.gltf` whose buffers live in sibling files cannot load from
/// bytes alone — that refusal names the sibling files and points at
/// [`load_from_path`].
//
// (No `#[must_use]`: `Result` is already `#[must_use]`, and an explicit
// attribute on top of it is clippy's `double_must_use`.)
pub fn load_from_bytes(bytes: &[u8]) -> Result<Vec<GltfMesh>, GltfError> {
    let gltf = gltf::Gltf::from_slice(bytes).map_err(map_parse_error)?;
    match gltf.blob.as_deref() {
        Some(blob) => {
            let buffer_count = gltf.document.buffers().count();
            if buffer_count != 1 {
                return Err(GltfError::Unsupported(format!(
                    "expected a single embedded buffer, found {buffer_count}"
                )));
            }
            load_core(&gltf.document, &[blob])
        }
        None => {
            let buffer_count = gltf.document.buffers().count();
            if buffer_count == 0 {
                return load_core(&gltf.document, &[]);
            }
            Err(GltfError::Unsupported(format!(
                "the .gltf document references {buffer_count} external buffer(s); \
                 the byte form cannot read sibling files, \
                 use load_from_path on the .gltf path"
            )))
        }
    }
}

/// Loads every mesh instance from a `.glb` or `.gltf` filesystem path.
///
/// A GLB reads whole and goes through [`load_from_bytes`], exactly as
/// before. A `.gltf` JSON document additionally reads each sibling `.bin`
/// the document names, resolved beside the file, and refuses an embedded
/// (`BIN` chunk) or data-URI buffer by name. An unreadable sibling is
/// [`GltfError::Io`]. Routing is by content magic, not by extension. (No
/// `#[must_use]`: `Result`.)
pub fn load_from_path(path: &Path) -> Result<Vec<GltfMesh>, GltfError> {
    let bytes = std::fs::read(path)
        .map_err(|error| GltfError::Io(format!("{}: {error}", path.display())))?;
    if bytes.starts_with(b"glTF") {
        return load_from_bytes(&bytes);
    }
    let gltf = gltf::Gltf::from_slice(&bytes).map_err(map_parse_error)?;
    let sibling_dir: &Path = path.parent().unwrap_or_else(|| Path::new(""));
    let mut owned: Vec<Vec<u8>> = Vec::new();
    for buffer in gltf.document.buffers() {
        match buffer.source() {
            Source::Bin => {
                return Err(GltfError::Unsupported(format!(
                    "buffer {} has no URI; only the GLB container carries an embedded buffer",
                    buffer.index()
                )));
            }
            Source::Uri(uri) => {
                if uri.starts_with("data:") {
                    return Err(GltfError::Unsupported(format!(
                        "buffer {} is a data URI; embedded base64 needs the `import` feature, \
                         which stays off",
                        buffer.index()
                    )));
                }
                let sibling = sibling_dir.join(uri);
                match std::fs::read(&sibling) {
                    Ok(contents) => owned.push(contents),
                    Err(error) => {
                        return Err(GltfError::Io(format!("{}: {error}", sibling.display())));
                    }
                }
            }
        }
    }
    let slices: Vec<&[u8]> = owned.iter().map(Vec::as_slice).collect();
    load_core(&gltf.document, &slices)
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

/// Converts a node's local transform to a [`Mat4`].
///
/// The crate's `Transform::matrix` already composes `T * R * S` from a
/// quaternion and passes a `Matrix` through, both in column-major order, so
/// [`Mat4::from_cols`] takes its sixteen floats verbatim. Total: every JSON
/// local form maps to exactly one matrix, and the hierarchy below decides
/// what the world means.
fn local_matrix(node: &gltf::Node<'_>) -> Mat4 {
    let matrix = node.transform().matrix();
    Mat4::from_cols([
        matrix[0][0],
        matrix[0][1],
        matrix[0][2],
        matrix[0][3], //
        matrix[1][0],
        matrix[1][1],
        matrix[1][2],
        matrix[1][3], //
        matrix[2][0],
        matrix[2][1],
        matrix[2][2],
        matrix[2][3], //
        matrix[3][0],
        matrix[3][1],
        matrix[3][2],
        matrix[3][3],
    ])
}

/// Loads every mesh instance from a parsed document and its buffer bytes.
///
/// The testable core both public forms share: `buffers` holds one byte slice
/// per document buffer, in index order — the GLB's BIN chunk or the `.gltf`'s
/// sibling files the caller already read, so no test here needs a
/// filesystem. Each slice must cover its buffer's declared length; a short
/// slice is [`GltfError::Unsupported`], not a panic in the crate's reader.
///
/// The default scene's hierarchy is walked depth-first from its roots (the
/// first scene when no default is set; every node as its own root when no
/// scene exists at all), accumulating `parent * local` worlds. A node with a
/// mesh becomes one [`GltfMesh`] per its one primitive; a node reached twice
/// is emitted once. Output is in node order regardless of the scene's root
/// order, which is what keeps the translation-only sedan byte-identical with
/// the flat iteration this replaces.
fn load_core(document: &gltf::Document, buffers: &[&[u8]]) -> Result<Vec<GltfMesh>, GltfError> {
    let buffer_count = document.buffers().count();
    if buffers.len() != buffer_count {
        return Err(GltfError::Unsupported(format!(
            "expected {buffer_count} buffer(s), found {}",
            buffers.len()
        )));
    }
    for (buffer, slice) in document.buffers().zip(buffers.iter()) {
        if slice.len() < buffer.length() {
            return Err(GltfError::Unsupported(format!(
                "buffer {} declares {} bytes but only {} were provided",
                buffer.index(),
                buffer.length(),
                slice.len()
            )));
        }
    }

    let mut roots: Vec<gltf::Node<'_>> = document
        .default_scene()
        .map(|scene| scene.nodes().collect())
        .unwrap_or_default();
    if roots.is_empty() {
        if let Some(scene) = document.scenes().next() {
            roots = scene.nodes().collect();
        }
    }

    // Each entry is a node with its parent's world; the node's own world is
    // `parent * local`, composed at pop time. Roots carry the identity.
    let mut stack: Vec<(gltf::Node<'_>, Mat4)> = roots
        .into_iter()
        .map(|node| (node, Mat4::identity()))
        .collect();
    let mut visited = vec![false; document.nodes().count()];
    let mut ordered: Vec<(usize, usize, GltfMesh)> = Vec::new();
    while let Some((node, parent_world)) = stack.pop() {
        let index = node.index();
        if visited.get(index).copied().unwrap_or(true) {
            continue;
        }
        if let Some(slot) = visited.get_mut(index) {
            *slot = true;
        }
        let world = parent_world.multiply(&local_matrix(&node));
        load_node_mesh(&node, &world, buffers, &mut ordered)?;
        for child in node.children() {
            stack.push((child, world));
        }
    }
    // Nodes with a mesh that no scene reaches keep the old flat behaviour:
    // their world is their local transform.
    for node in document.nodes() {
        if visited.get(node.index()).copied().unwrap_or(true) {
            continue;
        }
        if let Some(slot) = visited.get_mut(node.index()) {
            *slot = true;
        }
        load_node_mesh(&node, &local_matrix(&node), buffers, &mut ordered)?;
    }
    ordered.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    Ok(ordered.into_iter().map(|(_, _, part)| part).collect())
}

/// Loads one node's mesh, if it has one, into the output.
///
/// Skips mesh-less nodes (groups); refuses a mesh with zero or several
/// primitives by name, exactly as before. A translation-only world keeps the
/// old representation — local vertices under a translation matrix — and any
/// other world is baked with an identity placement (see the module docs).
fn load_node_mesh(
    node: &gltf::Node<'_>,
    world: &Mat4,
    buffers: &[&[u8]],
    ordered: &mut Vec<(usize, usize, GltfMesh)>,
) -> Result<(), GltfError> {
    let Some(part) = node.mesh() else {
        return Ok(());
    };
    let name = node
        .name()
        .or_else(|| part.name())
        .unwrap_or("")
        .to_string();
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
    let mut mesh = read_primitive(&primitive, buffers, &name)?;
    let material = primitive_material(&primitive);
    let transform = match world.as_translation() {
        Some(offset) => Mat4::identity().translated(offset),
        None => {
            bake_world(&mut mesh, world, &name)?;
            Mat4::identity()
        }
    };
    ordered.push((
        node.index(),
        0,
        GltfMesh {
            name,
            mesh,
            transform,
            material,
        },
    ));
    Ok(())
}

/// Bakes a world matrix into a mesh's vertices.
///
/// Positions go through the full 4x4; normals go through its
/// inverse-transpose ([`Mat4::transform_normal`]) and are renormalised by
/// [`MeshVertex::new`] below. A singular world keeps the file's normal
/// rather than inventing one; a world that turns a position into `w = 0` is
/// [`GltfError::Unsupported`], naming the mesh.
fn bake_world(mesh: &mut Mesh, world: &Mat4, name: &str) -> Result<(), GltfError> {
    for vertex in &mut mesh.vertices {
        let position = world.transform_point(vertex.position).ok_or_else(|| {
            GltfError::Unsupported(format!(
                "mesh {name:?} world transform turns a position into w = 0"
            ))
        })?;
        let normal = world
            .transform_normal(vertex.normal)
            .unwrap_or(vertex.normal);
        *vertex = MeshVertex::new(position, normal, vertex.uv);
    }
    Ok(())
}

/// Reads one primitive's base-color material.
///
/// Total: a missing material is the crate's default (opaque white, no
/// texture), an embedded buffer-view image or a data URI has no sibling file
/// and reads as no texture, and every other property is ignored — geometry
/// plus base color is the subset, recorded on [`GltfMaterial`].
fn primitive_material(primitive: &gltf::Primitive<'_>) -> GltfMaterial {
    let material = primitive.material();
    let factors = material.pbr_metallic_roughness();
    let base_color_texture = factors
        .base_color_texture()
        .map(|info| info.texture().source())
        .and_then(|image| match image.source() {
            gltf::image::Source::Uri { uri, .. } => file_name(uri).map(str::to_string),
            gltf::image::Source::View { .. } => None,
        });
    GltfMaterial {
        name: material.name().unwrap_or("").to_string(),
        base_color_texture,
        base_color_factor: factors.base_color_factor(),
    }
}

/// Takes a buffer or image URI down to the file name beside the model.
///
/// `"Textures/colormap.png"` reads as `"colormap.png"`, which is what the
/// demo resolves beside the model file. A data URI, an empty name or an
/// empty URI has no file and reads as `None`.
fn file_name(uri: &str) -> Option<&str> {
    if uri.starts_with("data:") {
        return None;
    }
    match uri.rsplit('/').next() {
        Some(name) if !name.is_empty() => Some(name),
        _ => None,
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
    buffers: &[&[u8]],
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

    let reader = primitive.reader(|buffer| buffers.get(buffer.index()).copied());
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

    /// A `.gltf` JSON document with an external buffer is not refused as a
    /// container; the byte form has no sibling files to read, so it names
    /// them and points at [`load_from_path`].
    #[test]
    fn json_bytes_name_the_sibling_buffer() {
        let error = load_from_bytes(json_doc(MESH_ONE, NODES, "[0,1]").as_bytes()).unwrap_err();
        assert!(
            matches!(&error, GltfError::Unsupported(message)
                if !message.contains("binary glTF")
                    && (message.contains("sibling") || message.contains("load_from_path"))),
            "expected an Unsupported naming the sibling buffer, got {error:?}"
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

    /// A document with an explicit scene-root list, for hierarchy fixtures:
    /// `document()` pins roots `[0, 1]`, which would also root a child.
    fn document_with_roots(meshes: &str, nodes: &str, accessors: &str, roots: &str) -> String {
        format!(
            concat!(
                r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":{roots}}}],"#,
                r#""nodes":{nodes},"meshes":{meshes},"accessors":{accessors},"#,
                r#""bufferViews":{views},"buffers":[{{"byteLength":102}}]}}"#
            ),
            nodes = nodes,
            meshes = meshes,
            accessors = accessors,
            views = VIEWS,
            roots = roots
        )
    }

    /// The same document as JSON (`.gltf`): one external buffer, `scene.bin`.
    fn json_doc(meshes: &str, nodes: &str, roots: &str) -> String {
        document_with_roots(meshes, nodes, ACCESSORS, roots).replace(
            r#""buffers":[{"byteLength":102}]"#,
            r#""buffers":[{"byteLength":102,"uri":"scene.bin"}]"#,
        )
    }

    /// A node rotated 180 degrees about z loads with its world baked into
    /// the vertices and an identity placement. The half-turn is exact in
    /// `f32`, so the baked positions and normals are hand-written literals,
    /// not approximations.
    #[test]
    fn a_rotated_node_bakes_its_world_into_vertices() {
        let nodes = r#"[{"mesh":0,"rotation":[0.0,0.0,1.0,0.0],"name":"spin"}]"#;
        let meshes = load_from_bytes(&glb(
            &document_with_roots(MESH_ONE, nodes, ACCESSORS, "[0]"),
            &triangle_bin([0, 1, 2]),
        ))
        .expect("a rotated node loads by baking");
        assert_eq!(meshes[0].transform, Mat4::identity());
        let positions: Vec<[f32; 3]> = meshes[0]
            .mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position)
            .collect();
        assert_eq!(
            positions,
            vec![[0.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, -1.0, 0.0]]
        );
        for vertex in &meshes[0].mesh.vertices {
            assert_eq!(vertex.normal, [0.0, 0.0, 1.0]);
        }
    }

    /// A uniformly scaled node loads baked: positions doubled, normals
    /// still up after `MeshVertex::new`, placement identity.
    #[test]
    fn a_scaled_node_bakes() {
        let nodes = r#"[{"mesh":0,"scale":[2.0,2.0,2.0],"name":"big"}]"#;
        let meshes = load_from_bytes(&glb(
            &document_with_roots(MESH_ONE, nodes, ACCESSORS, "[0]"),
            &triangle_bin([0, 1, 2]),
        ))
        .expect("a scaled node loads by baking");
        assert_eq!(meshes[0].transform, Mat4::identity());
        let positions: Vec<[f32; 3]> = meshes[0]
            .mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position)
            .collect();
        assert_eq!(
            positions,
            vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]]
        );
    }

    /// A matrix node carrying a pure translation loads as that translation,
    /// with local vertices — the translation fast path, not the bake.
    #[test]
    fn a_translation_matrix_node_loads_as_its_translation() {
        let nodes = concat!(
            r#"[{"mesh":0,"name":"matrix","matrix":"#,
            r#"[1.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0,0.0,0.0,1.0,0.0,1.0,2.0,3.0,1.0]}]"#
        );
        let meshes = load_from_bytes(&glb(
            &document_with_roots(MESH_ONE, nodes, ACCESSORS, "[0]"),
            &triangle_bin([0, 1, 2]),
        ))
        .expect("a translation matrix loads");
        assert_eq!(
            meshes[0].transform,
            Mat4::identity().translated([1.0, 2.0, 3.0])
        );
    }

    /// A matrix node carrying a rotation bakes its world and goes identity.
    #[test]
    fn a_rotation_matrix_node_bakes() {
        let nodes = concat!(
            r#"[{"mesh":0,"name":"spin","matrix":"#,
            r#"[-1.0,0.0,0.0,0.0,0.0,-1.0,0.0,0.0,0.0,0.0,1.0,0.0,5.0,0.0,0.0,1.0]}]"#
        );
        let meshes = load_from_bytes(&glb(
            &document_with_roots(MESH_ONE, nodes, ACCESSORS, "[0]"),
            &triangle_bin([0, 1, 2]),
        ))
        .expect("a rotation matrix loads by baking");
        assert_eq!(meshes[0].transform, Mat4::identity());
        let positions: Vec<[f32; 3]> = meshes[0]
            .mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position)
            .collect();
        assert_eq!(
            positions,
            vec![[5.0, 0.0, 0.0], [4.0, 0.0, 0.0], [5.0, -1.0, 0.0]]
        );
    }

    /// The child world is parent times local: a translated parent over a
    /// translated child places the part at the sum, still a translation.
    #[test]
    fn a_translation_hierarchy_accumulates() {
        let nodes = concat!(
            r#"[{"translation":[1.0,0.0,0.0],"children":[1],"name":"parent"},"#,
            r#"{"mesh":0,"translation":[0.0,2.0,0.0],"name":"child"}]"#
        );
        let meshes = load_from_bytes(&glb(
            &document_with_roots(MESH_ONE, nodes, ACCESSORS, "[0]"),
            &triangle_bin([0, 1, 2]),
        ))
        .expect("a hierarchy loads");
        assert_eq!(meshes.len(), 1);
        assert_eq!(
            meshes[0].transform,
            Mat4::identity().translated([1.0, 2.0, 0.0])
        );
    }

    /// A rotated child of a translated parent bakes the whole world:
    /// positions turned then moved, normals turned, placement identity.
    #[test]
    fn a_rotated_child_bakes_through_its_parent() {
        let nodes = concat!(
            r#"[{"translation":[1.0,0.0,0.0],"children":[1],"name":"parent"},"#,
            r#"{"mesh":0,"rotation":[0.0,0.0,1.0,0.0],"name":"child"}]"#
        );
        let meshes = load_from_bytes(&glb(
            &document_with_roots(MESH_ONE, nodes, ACCESSORS, "[0]"),
            &triangle_bin([0, 1, 2]),
        ))
        .expect("a rotated child loads by baking");
        assert_eq!(meshes[0].transform, Mat4::identity());
        let positions: Vec<[f32; 3]> = meshes[0]
            .mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position)
            .collect();
        assert_eq!(
            positions,
            vec![[1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, -1.0, 0.0]]
        );
    }

    /// One mesh whose primitive carries a material: a base-color texture and
    /// an explicit translucent factor. The texture URI resolves to the file
    /// name beside the model; the factor comes through verbatim.
    const MESH_ONE_MATERIAL: &str = concat!(
        r#"[{"name":"painted","primitives":[{"#,
        r#""attributes":{"POSITION":0,"NORMAL":1,"TEXCOORD_0":2},"indices":3,"mode":4,"#,
        r#""material":0}]}]"#
    );

    /// A document with one material, one texture and one image beside the
    /// usual fixture geometry.
    fn material_doc(nodes: &str, material: &str) -> String {
        format!(
            concat!(
                r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],"#,
                r#""nodes":{nodes},"meshes":{meshes},"materials":[{material}],"#,
                r#""textures":[{{"source":0}}],"images":[{{"uri":"Textures/part.png"}}],"#,
                r#""accessors":{accessors},"bufferViews":{views},"#,
                r#""buffers":[{{"byteLength":102,"uri":"scene.bin"}}]}}"#
            ),
            nodes = nodes,
            meshes = MESH_ONE_MATERIAL,
            material = material,
            accessors = ACCESSORS,
            views = VIEWS
        )
    }

    #[test]
    fn material_texture_file_and_factor_come_through() {
        let nodes = r#"[{"mesh":0,"name":"painted"}]"#;
        let material = r#"{"name":"paint","pbrMetallicRoughness":{"baseColorFactor":[0.5,0.25,0.125,0.5],"baseColorTexture":{"index":0}}}"#;
        let parsed =
            gltf::Gltf::from_slice(material_doc(nodes, material).as_bytes()).expect("valid JSON");
        let bin = triangle_bin([0, 1, 2]);
        let meshes = load_core(&parsed.document, &[&bin]).expect("material doc loads");
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].material.name, "paint");
        assert_eq!(
            meshes[0].material.base_color_texture.as_deref(),
            Some("part.png")
        );
        assert_eq!(
            meshes[0].material.base_color_factor,
            [0.5, 0.25, 0.125, 0.5]
        );
    }

    /// A primitive with no material reads as the crate's default: opaque
    /// white with no texture, the tint the demo can multiply blindly.
    #[test]
    fn a_primitive_without_a_material_is_opaque_white() {
        let parsed = gltf::Gltf::from_slice(json_doc(MESH_ONE, NODES, "[0,1]").as_bytes())
            .expect("valid JSON");
        let bin = triangle_bin([0, 1, 2]);
        let meshes = load_core(&parsed.document, &[&bin]).expect("plain doc loads");
        assert_eq!(meshes[0].material.name, "");
        assert_eq!(meshes[0].material.base_color_texture, None);
        assert_eq!(meshes[0].material.base_color_factor, [1.0, 1.0, 1.0, 1.0]);
    }

    /// Two external buffers — positions in the first, everything else in
    /// the second — load through the slice core, which is how a `.gltf`
    /// with sibling files reaches geometry without a filesystem in tests.
    const VIEWS_TWO_BUFFERS: &str = concat!(
        r#"[{"buffer":0,"byteOffset":0,"byteLength":36},"#,
        r#"{"buffer":1,"byteOffset":0,"byteLength":36},"#,
        r#"{"buffer":1,"byteOffset":36,"byteLength":24},"#,
        r#"{"buffer":1,"byteOffset":60,"byteLength":6}]"#
    );

    /// A document splitting the fixture geometry across two buffers.
    fn two_buffer_doc() -> String {
        format!(
            concat!(
                r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],"#,
                r#""nodes":[{{"mesh":0,"name":"split"}}],"meshes":{meshes},"accessors":{accessors},"#,
                r#""bufferViews":{views},"buffers":[{{"byteLength":36,"uri":"a.bin"}},"#,
                r#"{{"byteLength":66,"uri":"b.bin"}}]}}"#
            ),
            meshes = MESH_ONE,
            accessors = ACCESSORS,
            views = VIEWS_TWO_BUFFERS
        )
    }

    #[test]
    fn two_external_buffers_load_through_the_core() {
        let parsed = gltf::Gltf::from_slice(two_buffer_doc().as_bytes()).expect("valid JSON");
        let bin = triangle_bin([0, 1, 2]);
        let meshes =
            load_core(&parsed.document, &[&bin[..36], &bin[36..]]).expect("two buffers load");
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].mesh.vertices.len(), 3);
        assert_eq!(meshes[0].mesh.indices, vec![0, 1, 2]);
    }

    /// Fewer slices than document buffers is refused, naming the counts —
    /// a short read the crate's accessor iterator must never see.
    #[test]
    fn a_missing_buffer_slice_is_unsupported() {
        let parsed = gltf::Gltf::from_slice(two_buffer_doc().as_bytes()).expect("valid JSON");
        let bin = triangle_bin([0, 1, 2]);
        let error = load_core(&parsed.document, &[&bin[..]]).unwrap_err();
        assert!(
            matches!(&error, GltfError::Unsupported(message) if message.contains("2")),
            "expected an Unsupported naming the buffer count, got {error:?}"
        );
    }
}
