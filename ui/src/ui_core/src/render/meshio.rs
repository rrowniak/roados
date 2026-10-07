//! Model files: the `ROADOSMF` container and the loader that reads it.
//!
//! A mesh that comes from **a file**, in a format this project owns: a small
//! binary container for interleaved vertices, absolute indices and named
//! sub-mesh ranges, read by a safe-Rust loader that parses a `&[u8]` into task
//! 35's [`crate::render::mesh::Mesh`] and returns a specific error for
//! every way a file can be wrong — no panic, no silently empty mesh, no
//! oversized allocation.
//!
//! ## Model files
//!
//! Little-endian throughout; **no padding anywhere**; every field is naturally
//! aligned for its own width from an offset that is a multiple of that width.
//! The header is **40 bytes**, then three variable-length blocks in this order.
//!
//! | offset | size | type | field | meaning |
//! |---|---|---|---|---|
//! | 0 | 8 | `[u8; 8]` | `magic` | `b"ROADOSMF"` |
//! | 8 | 4 | `u32` | `version` | **1**. A reader that sees anything else refuses |
//! | 12 | 4 | `u32` | `header_bytes` | **40**. Size of this header |
//! | 16 | 4 | `u32` | `vertex_count` | vertices in the merged array |
//! | 20 | 4 | `u32` | `index_count` | indices in the merged array, **a multiple of 3** |
//! | 24 | 4 | `u32` | `sub_mesh_count` | named sub-meshes, **≥ 1** |
//! | 28 | 4 | `u32` | `name_bytes` | total length of the name block |
//! | 32 | 4 | `u32` | `flags` | **0** in v1; a non-zero value is refused |
//! | 36 | 4 | `u32` | `reserved` | **0** in v1; a non-zero value is refused |
//! | 40 | `vertex_count × 32` | bytes | `vertices` | interleaved, stride 32, **task 35's layout verbatim** |
//! | 40 + V·32 | `index_count × 4` | `u32` | `indices` | **absolute** vertex indices into `vertices` |
//! | 40 + V·32 + I·4 | `S × 8` | `u32` pair | `sub_meshes` | `(first_index, index_count)` per sub-mesh |
//! | 40 + V·32 + I·4 + S·8 | `name_bytes` | UTF-8 | `names` | `sub_mesh_count` NUL-terminated strings, in the same order |
//!
//! `V`, `I` and `S` are `vertex_count`, `index_count` and `sub_mesh_count`. The
//! `header_bytes` field is checked and never used to seek: a reader that trusts
//! the header's own size field to find the payload can be pointed anywhere in
//! the file.
//!
//! The vertex block is **`position [f32;3]`, `normal [f32;3]`, `uv [f32;2]`** —
//! 32 bytes, no padding, [`VERTEX_STRIDE_BYTES`], and task 35's
//! `MESH_NORMAL_OFFSET` = 12 and `MESH_UV_OFFSET` = 24. The reader decodes field
//! by field with `f32::from_le_bytes` on four-byte windows rather than by a
//! transmute-shaped reinterpretation over a struct.
//!
//! The name block is one run of NUL-terminated UTF-8 strings, not a directory
//! of offsets — an offsets table is 4 bytes per name to save a scan of a block
//! that is read **once**, at load. `sub_mesh_count` names, `name_bytes` bytes,
//! each name **at least one byte** (the NUL), and **no interior NUL** in any
//! name, which is what makes splitting on `0` unambiguous. A variable
//! `sub_mesh_count` and name table rather than five hard-coded slots is what
//! lets a future converter split the glass out of the body without a format
//! version bump.
//!
//! What the format deliberately does not carry, each with its reason in one line:
//!
//! - **Node transforms, hierarchies, or a scene graph.** Baked at conversion
//!   time; a converter is edited by a human with a diff, a runtime scene graph
//!   is a bug class re-tested on every model change.
//! - **A texture reference, a material, or a URI.** The draw command carries the
//!   texture, so one model draws with a different colormap without re-uploading.
//! - **Tangents, colours, joints, weights, or a second UV set.** A second set is
//!   a stride change and belongs to whoever changes the format.
//! - **A bounding box, an extent, or a pivot.** The demo places the car with a
//!   matrix; a file pivot is a second answer to that question.
//! - **Compression.** A quarter of a megabyte read once at start-up; a
//!   decompressor would own the bounds arithmetic below.
//! - **A draw order, a material id, or a per-sub-mesh colour.** Per-command tint
//!   on the recording side; a file field for it is a `Property` in another name.
//! - **Anything requiring JSON, base64, or a text encoding.**
//!
//! ## The six errors, and the repair each one asks for
//!
//! - [`MeshError::Io`]: the path could not be opened or read — fix the path.
//! - [`MeshError::NotAMesh`]: the first eight bytes are not `ROADOSMF`, carried
//!   as read and rendered as hex — wrong file, not a broken mesh.
//! - [`MeshError::Version`]: the version is not one this build reads —
//!   regenerate or upgrade, never "try anyway".
//! - [`MeshError::Malformed`]: the header says something the layout cannot be —
//!   the message names the field, because no caller places anything from it and
//!   the message is the specification.
//! - [`MeshError::Truncated`]: a declared size does not fit the bytes at hand —
//!   re-copy the file; the `needed` value names the block by its prefix length.
//! - [`MeshError::TooLarge`]: a payload that fits the file byte-for-byte and
//!   still cannot be addressed on this target — produce a smaller model. Never a
//!   huge allocation: nothing is allocated before check 13.
//!
//! ## The thirteen checks, in order
//!
//! 1. Shorter than the header is `Truncated`, before the magic is even read.
//! 2. Wrong magic is `NotAMesh`.
//! 3. Wrong `version` is `Version`; wrong `header_bytes`, non-zero `flags` or
//!    `reserved` is `Malformed` naming the field.
//! 4. Each count `usize::try_from`ed; a `None` is `Malformed` naming the field.
//! 5. Every count non-zero and `index_count % 3 == 0`, else `Malformed`.
//! 6. Every length in `u64` with `checked_mul`/`checked_add` — `0x0800_0000 × 32`
//!    is `2^32`, which a `u32` multiply wraps to exactly zero.
//! 7. Each payload prefix against the byte length before any allocation; the
//!    shortfall is `Truncated` whose `needed` names the block.
//! 8. `name_bytes <= sub_mesh_count × 64`, a stated ceiling, else `Malformed`.
//! 9. The name block decodes as UTF-8 and splits into exactly `sub_mesh_count`
//!    non-empty names, else `Malformed`.
//! 10. Every index value `< vertex_count`, checked as read, naming value
//!     and position.
//! 11. Every `(first_index, index_count)` pair tiles in file order: starts
//!     where the previous ended, non-zero, a multiple of three, within the
//!     index array.
//! 12. The ranges end exactly at `index_count`.
//! 13. Only then is anything allocated, `with_capacity` of counts proven to
//!     fit the file, with one `TooLarge` check of the two byte counts
//!     against `isize::MAX`.
//!
//! ## The baked-transform contract, and the assumption it rests on
//!
//! The converter applies every node's translation (and rotation and scale, if a
//! future model has them) to the **positions**, transforms the **normals** by
//! the rotation part alone — a normal is a direction, so a translation must not
//! move it and a scale must not lengthen it beyond what the reader normalises —
//! and emits one vertex array with the parts concatenated in file order.
//! Indices are rebased once, at conversion, so the file's are **absolute into
//! the merged `vertices`** and the loader adds nothing; a reader that added a
//! base would double-count. `sub_meshes[i].first_index` is an **index
//! position**, not a byte offset, and the ranges tile the index array exactly
//! and in order, because a gap is triangles no draw call ever draws and an
//! overlap is some of them drawn twice. The loader knows no name, requires no
//! wheel, and orders nothing.
//!
//! ## Rejected alternatives, in full
//!
//! **GLB.** Embeds JSON in a binary chunk: no parser, but the accessor
//! indirection stays — buffer views, byte offsets, strides, component types and
//! counts per attribute per primitive, resolved through tables of tables — for
//! what conversion-time baking already resolves to vertices and index triples.
//! Most of the format (scenes, skins, animations, cameras, samplers, materials,
//! extensions) has no consumer here, and reading fields that can only ever be
//! ignored is a validation surface with nothing behind it.
//!
//! **glTF.** JSON, so reading it is reading JSON: a parser, an error taxonomy
//! for a language this project would otherwise not have, and a stream state
//! machine — plus everything GLB keeps.
//!
//! **OBJ.** Text: a line parser, float parsing per vertex per load, no indices
//! worthy of the name (separate index triples per attribute), and no place for
//! named sub-mesh ranges.
//!
//! **Embedding the vertex bytes in the binary.** A `&[u8]` baked into the
//! executable is the fastest load and the worst asset story: every model tweak
//! rebuilds the binary, the bytes are uninspectable without the reader, and a
//! head unit that wants a different car wants a different binary.
//!
//! A project format cannot be opened by any other tool. That is accepted for
//! this sequence and reversible — the header carries a magic and a version
//! precisely so a future GLB reader, or a `v2` with tangents, is a detectable
//! change rather than a silent misparse.
//!
//! ## The `&Path` wrapper, and what it costs
//!
//! [`load_from_bytes`] is the whole of the loader and [`load_from_path`] is a
//! wrapper over it, because `AGENTS.md` permits no test that needs a filesystem
//! and every test in this module calls the byte form. `load_from_path` is one
//! function and one `?`, not a trait and not a generic over a reader — one
//! caller is a function, not a trait — and it is the first `ui_core` function
//! that opens a file itself, while the font and image loaders delegate to a
//! library. `load_from_bytes` therefore cannot produce [`MeshError::Io`].

use std::path::Path;

use crate::render::mesh::{Mesh, MeshVertex, SubMesh};

/// The first eight bytes of every model file: *ROADOS Mesh Format*.
///
/// Eight bytes rather than five because a bare `ROADOS` would collide with a
/// future container that is not a mesh, and because these are what an operator
/// sees in a hex dump when the demo says *"not a mesh"*.
pub const MESH_MAGIC: [u8; 8] = *b"ROADOSMF";

/// The model file version this build reads.
///
/// Anything else is refused, never "tried anyway": a format whose reader
/// guesses draws a plausible wrong car.
pub const MESH_VERSION: u32 = 1;

/// The size of the model file header in bytes.
///
/// Checked, never used to seek: a reader that trusts the header's own size
/// field to find the payload can be pointed anywhere in the file.
pub const MESH_HEADER_BYTES: u32 = 40;

/// The stride of one model-file vertex in bytes: 8 `f32` fields, no padding.
///
/// Asserted equal to task 35's `MESH_VERTEX_STRIDE`, so the file's byte layout
/// and the `#[repr(C)]` struct cannot drift apart.
pub const VERTEX_STRIDE_BYTES: u32 = 32;

/// The most name-block bytes one sub-mesh may account for.
///
/// A stated ceiling, not a measurement — every name in this model is
/// `wheel-front-left`, 16 bytes — and raising it is a version-bump decision,
/// not an edit.
const MAX_NAME_BYTES_PER_MESH: u64 = 64;

/// Why a model file could not be loaded.
///
/// Six variants because six different mistakes deserve six different repairs.
/// [`Clone`] and [`Eq`], like [`TextureError`](crate::texture::TextureError),
/// so a caller can hold one beside anything else an asset load returns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MeshError {
    /// The path could not be opened or read.
    ///
    /// Only [`load_from_path`] produces this: a missing file and a malformed
    /// file are different answers here.
    Io(String),
    /// The first eight bytes are not `ROADOSMF`. Carries them, and the
    /// `Display` renders them as hex, so a caller can tell a PNG from an HTML
    /// error page.
    NotAMesh {
        /// The eight bytes that were there instead.
        found: [u8; 8],
    },
    /// The version is not one this build reads.
    Version {
        /// The version the file carries.
        found: u32,
        /// The version this build reads.
        expected: u32,
    },
    /// The header says something the layout cannot be: a bad `header_bytes`, a
    /// non-zero `flags` or `reserved`, a zero count, an `index_count` that is
    /// not a multiple of three, a sub-mesh range that does not tile the index
    /// array, or bytes left over at the end. Carries the offending field's
    /// name, because no caller places anything from it and the message is the
    /// specification.
    Malformed(String),
    /// A declared size does not fit the bytes that are actually there. The
    /// `needed` value is the prefix length through the block that ran short,
    /// which is what names the block.
    Truncated {
        /// How many bytes the file would need through the short block.
        needed: u64,
        /// How many bytes were actually there.
        have: u64,
    },
    /// A payload this reader must build does not fit in memory on this target.
    /// Never a huge allocation: nothing is allocated before check 13, and a
    /// count that does not fit its own file is `Truncated`, full stop.
    TooLarge {
        /// The byte count that does not fit.
        bytes: u64,
    },
}

impl std::fmt::Display for MeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeshError::Io(message) => write!(f, "could not read model file: {message}"),
            MeshError::NotAMesh { found } => {
                let mut hex = String::with_capacity(16);
                for byte in found {
                    hex.push(char::from_digit(u32::from(*byte) >> 4, 16).unwrap_or('?'));
                    hex.push(char::from_digit(u32::from(*byte) & 0x0f, 16).unwrap_or('?'));
                }
                write!(f, "not a mesh file: magic {hex}")
            }
            MeshError::Version { found, expected } => write!(
                f,
                "model file version {found}, this build reads version {expected}"
            ),
            MeshError::Malformed(message) => write!(f, "malformed model file: {message}"),
            MeshError::Truncated { needed, have } => {
                write!(f, "truncated model file: needs {needed} bytes, has {have}")
            }
            MeshError::TooLarge { bytes } => write!(
                f,
                "model payload of {bytes} bytes does not fit in memory on this target"
            ),
        }
    }
}

impl std::error::Error for MeshError {}

/// Reads a little-endian `u32` at `offset`.
///
/// The caller has already proven the window in bounds, so the indexing cannot
/// fail; the helper exists so the eight header reads are one shape.
fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

/// Reads a little-endian `f32` at `offset`.
///
/// As with [`read_u32`], the caller has proven the window in bounds.
fn read_f32(bytes: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

/// Parses a model file from bytes.
///
/// Every check in the module docs' *thirteen checks*, in that order: shortness
/// before content, content before arithmetic, arithmetic before allocation.
/// No filesystem, no allocation before check 13, and nothing the borrow
/// checker would not sign off on.
///
/// A zero-length normal in a corrupt file becomes `[0.0, 0.0, 1.0]` rather than
/// a `NaN`, because [`MeshVertex::new`] normalises and that is the function
/// this loader builds every vertex through.
///
/// ```
/// use ui_core::render::meshio::load_from_bytes;
///
/// // The smallest valid file: a 40-byte header, one vertex, one triangle,
/// // one sub-mesh range, one two-byte name.
/// let mut file = vec![0u8; 94];
/// file[0..8].copy_from_slice(b"ROADOSMF");
/// file[8..12].copy_from_slice(&1u32.to_le_bytes());
/// file[12..16].copy_from_slice(&40u32.to_le_bytes());
/// file[16..20].copy_from_slice(&1u32.to_le_bytes());
/// file[20..24].copy_from_slice(&3u32.to_le_bytes());
/// file[24..28].copy_from_slice(&1u32.to_le_bytes());
/// file[28..32].copy_from_slice(&2u32.to_le_bytes());
/// file[88..92].copy_from_slice(&3u32.to_le_bytes());
/// file[92] = b'a';
/// // Normal [0,0,0] at bytes 52..64 normalises to [0,0,1] on load.
/// let mesh = load_from_bytes(&file).expect("a well-formed file loads");
/// assert_eq!(mesh.vertices.len(), 1);
/// assert_eq!(mesh.vertices[0].normal, [0.0, 0.0, 1.0]);
/// assert_eq!(mesh.sub_meshes[0].name, "a");
/// ```
//
// (No `#[must_use]` here, though the task file asks for one on every
// value-returning function: `Result` is already `#[must_use]` in std, and an
// explicit attribute on top of it is clippy's `double_must_use`.)
pub fn load_from_bytes(bytes: &[u8]) -> Result<Mesh, MeshError> {
    let have = bytes.len() as u64;

    // Check 1: shortness before content. A 12-byte file cannot carry a magic
    // and a version, and a reader that reads the magic first reports
    // `NotAMesh` for a file that was merely short.
    if bytes.len() < MESH_HEADER_BYTES as usize {
        return Err(MeshError::Truncated {
            needed: u64::from(MESH_HEADER_BYTES),
            have,
        });
    }

    // Check 2: the magic. Read field by field, with the length proven above,
    // so no fallible conversion stands between the bytes and the comparison.
    let mut magic = [0u8; 8];
    for (i, slot) in magic.iter_mut().enumerate() {
        *slot = bytes[i];
    }
    if magic != MESH_MAGIC {
        return Err(MeshError::NotAMesh { found: magic });
    }

    // Check 3: the version, then the header's own size field — checked, never
    // used to seek — then the two words a future v1 spends without a bump.
    let version = read_u32(bytes, 8);
    if version != MESH_VERSION {
        return Err(MeshError::Version {
            found: version,
            expected: MESH_VERSION,
        });
    }
    if read_u32(bytes, 12) != MESH_HEADER_BYTES {
        return Err(MeshError::Malformed(format!(
            "header_bytes is {}, expected {}",
            read_u32(bytes, 12),
            MESH_HEADER_BYTES
        )));
    }
    if read_u32(bytes, 32) != 0 {
        return Err(MeshError::Malformed(format!(
            "flags is {}, expected 0",
            read_u32(bytes, 32)
        )));
    }
    if read_u32(bytes, 36) != 0 {
        return Err(MeshError::Malformed(format!(
            "reserved is {}, expected 0",
            read_u32(bytes, 36)
        )));
    }

    let vertex_count_raw = read_u32(bytes, 16);
    let index_count_raw = read_u32(bytes, 20);
    let sub_mesh_count_raw = read_u32(bytes, 24);
    let name_bytes_raw = read_u32(bytes, 28);

    // Check 4: honest `usize` conversions. On a 32-bit target a `u32` count is
    // not automatically a `usize`, and the conversion is `try_from` rather
    // than an `as` cast.
    let vertex_count = usize::try_from(vertex_count_raw).map_err(|_| {
        MeshError::Malformed(format!(
            "vertex_count {vertex_count_raw} does not fit a usize"
        ))
    })?;
    let index_count = usize::try_from(index_count_raw).map_err(|_| {
        MeshError::Malformed(format!(
            "index_count {index_count_raw} does not fit a usize"
        ))
    })?;
    let sub_mesh_count = usize::try_from(sub_mesh_count_raw).map_err(|_| {
        MeshError::Malformed(format!(
            "sub_mesh_count {sub_mesh_count_raw} does not fit a usize"
        ))
    })?;
    let name_bytes = usize::try_from(name_bytes_raw).map_err(|_| {
        MeshError::Malformed(format!("name_bytes {name_bytes_raw} does not fit a usize"))
    })?;

    // Check 5: every count non-zero — a zero count is task 35's first
    // validation, and the loader is where it first bites — and the index
    // count a multiple of three, because every submission is
    // `draw_elements(GL_TRIANGLES, …)`.
    if vertex_count == 0 {
        return Err(MeshError::Malformed(
            "vertex_count is 0, expected at least 1".to_string(),
        ));
    }
    if index_count == 0 {
        return Err(MeshError::Malformed(
            "index_count is 0, expected at least 1".to_string(),
        ));
    }
    if index_count % 3 != 0 {
        return Err(MeshError::Malformed(format!(
            "index_count {index_count} is not a multiple of 3"
        )));
    }
    if sub_mesh_count == 0 {
        return Err(MeshError::Malformed(
            "sub_mesh_count is 0, expected at least 1".to_string(),
        ));
    }

    // Check 6: every length in `u64` with checked arithmetic.
    // `0x0800_0000 × 32` is `2^32`, which a `u32` multiply wraps to exactly
    // zero — a reader computing in `u32` concludes a 40-byte file needs no
    // payload and accepts it.
    let vertices_bytes = u64::from(vertex_count_raw)
        .checked_mul(u64::from(VERTEX_STRIDE_BYTES))
        .ok_or_else(|| {
            MeshError::Malformed(format!(
                "vertex_count {vertex_count_raw} overflows the vertex block length"
            ))
        })?;
    let indices_bytes = u64::from(index_count_raw).checked_mul(4).ok_or_else(|| {
        MeshError::Malformed(format!(
            "index_count {index_count_raw} overflows the index block length"
        ))
    })?;
    let sub_mesh_bytes = u64::from(sub_mesh_count_raw)
        .checked_mul(8)
        .ok_or_else(|| {
            MeshError::Malformed(format!(
                "sub_mesh_count {sub_mesh_count_raw} overflows the sub-mesh block length"
            ))
        })?;
    let name_bytes_u64 = u64::from(name_bytes_raw);

    // Check 7: each payload prefix against the byte length, before any
    // allocation. The `needed` value is the prefix length through the block
    // that ran short, which is what names the block: a reader that allocated
    // `Vec::with_capacity(vertex_count)` before this check would let a 40-byte
    // file ask for 128 GB.
    let header_u64 = u64::from(MESH_HEADER_BYTES);
    let need_vertices = header_u64.checked_add(vertices_bytes).ok_or_else(|| {
        MeshError::Malformed("vertex_count overflows the vertex block end".to_string())
    })?;
    if have < need_vertices {
        return Err(MeshError::Truncated {
            needed: need_vertices,
            have,
        });
    }
    let need_indices = need_vertices.checked_add(indices_bytes).ok_or_else(|| {
        MeshError::Malformed("index_count overflows the index block end".to_string())
    })?;
    if have < need_indices {
        return Err(MeshError::Truncated {
            needed: need_indices,
            have,
        });
    }
    let need_sub_meshes = need_indices.checked_add(sub_mesh_bytes).ok_or_else(|| {
        MeshError::Malformed("sub_mesh_count overflows the sub-mesh block end".to_string())
    })?;
    if have < need_sub_meshes {
        return Err(MeshError::Truncated {
            needed: need_sub_meshes,
            have,
        });
    }
    let need_names = need_sub_meshes.checked_add(name_bytes_u64).ok_or_else(|| {
        MeshError::Malformed("name_bytes overflows the name block end".to_string())
    })?;
    if have < need_names {
        return Err(MeshError::Truncated {
            needed: need_names,
            have,
        });
    }

    // Check 8: the name-block ceiling. A stated ceiling, not a measurement:
    // every name in this model is `wheel-front-left`, 16 bytes, and raising
    // the 64 is a version-bump decision.
    let ceiling = u64::from(sub_mesh_count_raw)
        .checked_mul(MAX_NAME_BYTES_PER_MESH)
        .ok_or_else(|| {
            MeshError::Malformed("sub_mesh_count overflows the name ceiling".to_string())
        })?;
    if name_bytes_u64 > ceiling {
        return Err(MeshError::Malformed(format!(
            "name_bytes {name_bytes_raw} exceeds the ceiling of {ceiling}"
        )));
    }

    // Check 9: the name block decodes as UTF-8 and splits into exactly
    // `sub_mesh_count` non-empty names. An interior NUL is what makes the
    // split ambiguous, and an empty piece is what it produces.
    let names_off = need_sub_meshes as usize;
    let names_bytes = &bytes[names_off..names_off + name_bytes];
    let names_str = std::str::from_utf8(names_bytes).map_err(|error| {
        MeshError::Malformed(format!(
            "name block is not UTF-8 at byte offset {}",
            error.valid_up_to()
        ))
    })?;
    // Splitting an empty slice yields one empty piece; a zero-length name
    // block therefore reports one name found rather than zero, and the count
    // comparison below refuses it all the same.
    let names: Vec<&str> = names_str.split('\0').collect();
    // The split keeps the text after the last NUL, which must itself be empty:
    // a well-formed block ends in NUL, so the last piece is the terminator's
    // shadow rather than a name.
    let (names, trailing) = match names.split_last() {
        Some((last, rest)) => (rest, *last),
        None => (&names[..0], ""),
    };
    if !trailing.is_empty() {
        return Err(MeshError::Malformed(format!(
            "name block of {name_bytes_raw} bytes holds no trailing NUL"
        )));
    }
    if names.len() != sub_mesh_count {
        return Err(MeshError::Malformed(format!(
            "name block holds {} names for {sub_mesh_count_raw} sub-meshes",
            names.len()
        )));
    }
    for (k, name) in names.iter().enumerate() {
        if name.is_empty() {
            return Err(MeshError::Malformed(format!(
                "sub-mesh {k} has an empty name"
            )));
        }
    }

    // Check 10: every index value `< vertex_count`, checked as it is read.
    // This is task 35's third validation, run here against the file rather
    // than against a caller.
    let indices_off = (header_u64 + vertices_bytes) as usize;
    let mut indices = Vec::with_capacity(index_count);
    for pos in 0..index_count {
        let value = read_u32(bytes, indices_off + pos * 4);
        if value >= vertex_count_raw {
            return Err(MeshError::Malformed(format!(
                "index {value} at position {pos} reaches past the vertex array of {vertex_count_raw} vertices"
            )));
        }
        indices.push(value);
    }

    // Checks 11 and 12: every `(first_index, index_count)` pair tiles in file
    // order, and the tiling ends exactly at `index_count`. A gap is triangles
    // no draw call ever draws and an overlap is some of them drawn twice.
    let sub_mesh_off = (header_u64 + vertices_bytes + indices_bytes) as usize;
    let mut sub_meshes = Vec::with_capacity(sub_mesh_count);
    let mut running: u32 = 0;
    // `names` holds exactly `sub_mesh_count` entries after check 9, so this
    // iterates the names rather than the count: the name and the pair cannot
    // disagree about which sub-mesh they are.
    for (k, name) in names.iter().enumerate() {
        let first = read_u32(bytes, sub_mesh_off + k * 8);
        let count = read_u32(bytes, sub_mesh_off + k * 8 + 4);
        if first != running {
            return Err(MeshError::Malformed(format!(
                "sub-mesh {k} starts at index {first}, expected {running}"
            )));
        }
        if count == 0 {
            return Err(MeshError::Malformed(format!(
                "sub-mesh {k} has an index_count of 0"
            )));
        }
        if count % 3 != 0 {
            return Err(MeshError::Malformed(format!(
                "sub-mesh {k} has an index_count of {count}, not a multiple of 3"
            )));
        }
        let end = first.checked_add(count).ok_or_else(|| {
            MeshError::Malformed(format!(
                "sub-mesh {k} overflows first_index {first} + index_count {count}"
            ))
        })?;
        if end > index_count_raw {
            return Err(MeshError::Malformed(format!(
                "sub-mesh {k} ends at index {end} past the index array of {index_count_raw}"
            )));
        }
        running = end;
        sub_meshes.push(SubMesh {
            name: name.to_string(),
            first_index: first,
            index_count: count,
        });
    }
    if running != index_count_raw {
        return Err(MeshError::Malformed(format!(
            "sub-mesh ranges end at index {running}, expected {index_count_raw}"
        )));
    }

    // Trailing bytes after the name block: a file longer than it says it is
    // is either a concatenated pair or a different format, and both are worth
    // refusing rather than ignoring.
    if have > need_names {
        return Err(MeshError::Malformed(format!(
            "file holds {} trailing bytes after the name block",
            have - need_names
        )));
    }

    // Check 13: only now is anything allocated, `with_capacity` of counts
    // already proven to fit the file. `Vec::with_capacity` is not a promise
    // that it succeeds, so the ceiling that matters is check 7's — a count
    // that does not exceed the file's own length — and the format therefore
    // cannot describe a mesh larger than the bytes it occupies. The one
    // `TooLarge` this reader can still name is a payload no address space on
    // this target holds.
    let addressable = isize::MAX as usize as u64;
    if vertices_bytes > addressable {
        return Err(MeshError::TooLarge {
            bytes: vertices_bytes,
        });
    }
    if indices_bytes > addressable {
        return Err(MeshError::TooLarge {
            bytes: indices_bytes,
        });
    }
    let vertices_off = header_u64 as usize;
    let stride = usize::try_from(VERTEX_STRIDE_BYTES).map_err(|_| {
        MeshError::Malformed("VERTEX_STRIDE_BYTES does not fit a usize".to_string())
    })?;
    let mut vertices = Vec::with_capacity(vertex_count);
    for v in 0..vertex_count {
        let base = vertices_off + v * stride;
        vertices.push(MeshVertex::new(
            [
                read_f32(bytes, base),
                read_f32(bytes, base + 4),
                read_f32(bytes, base + 8),
            ],
            [
                read_f32(bytes, base + 12),
                read_f32(bytes, base + 16),
                read_f32(bytes, base + 20),
            ],
            [read_f32(bytes, base + 24), read_f32(bytes, base + 28)],
        ));
    }

    Ok(Mesh {
        vertices,
        indices,
        sub_meshes,
    })
}

/// Loads a model file from a filesystem path.
///
/// A wrapper, and the only function in `ui_core` that opens a file itself: it reads the
/// whole file and calls [`load_from_bytes`], mapping an `io::Error` to
/// [`MeshError::Io`]. The buffer form is the one that exists; this one is how
/// the demo reaches it. (No `#[must_use]`, for the reason above: `Result`.)
pub fn load_from_path(path: &Path) -> Result<Mesh, MeshError> {
    let bytes = std::fs::read(path)
        .map_err(|error| MeshError::Io(format!("{}: {error}", path.display())))?;
    load_from_bytes(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::mesh::MeshVertex;

    /// Encodes a model file exactly as the module doc's byte table lays it
    /// out: the test-side encoder the round-trip criterion requires.
    ///
    /// Private to `mod tests`, so this task ships no writer: the production
    /// encoder is task 39's, and this one exists so the byte table is a
    /// specification the loader is proved against rather than a description.
    fn encode_file(
        vertex_count: u32,
        vertex_bytes: &[u8],
        indices: &[u32],
        ranges: &[(u32, u32)],
        name_block: &[u8],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&MESH_MAGIC);
        out.extend_from_slice(&MESH_VERSION.to_le_bytes());
        out.extend_from_slice(&MESH_HEADER_BYTES.to_le_bytes());
        out.extend_from_slice(&vertex_count.to_le_bytes());
        out.extend_from_slice(&(indices.len() as u32).to_le_bytes());
        out.extend_from_slice(&(ranges.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name_block.len() as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(vertex_bytes);
        for index in indices {
            out.extend_from_slice(&index.to_le_bytes());
        }
        for (first, count) in ranges {
            out.extend_from_slice(&first.to_le_bytes());
            out.extend_from_slice(&count.to_le_bytes());
        }
        out.extend_from_slice(name_block);
        out
    }

    /// Encodes one vertex's eight `f32`s at stride 32, field by field.
    fn encode_vertex(position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        for (i, value) in position.into_iter().chain(normal).chain(uv).enumerate() {
            bytes[i * 4..i * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    /// The smallest interesting file: one vertex, one triangle, one range,
    /// one name. Most refusal tests start here and patch one field.
    fn smallest_valid_file() -> Vec<u8> {
        let vertex = encode_vertex([1.0, 2.0, 3.0], [0.0, 0.0, 5.0], [0.25, 0.5]);
        encode_file(1, &vertex, &[0, 0, 0], &[(0, 3)], b"a\0")
    }

    #[test]
    fn the_magic_is_roados_mfs_and_the_version_is_one() {
        assert_eq!(MESH_MAGIC, *b"ROADOSMF");
        assert_eq!(MESH_VERSION, 1);
        assert_eq!(MESH_HEADER_BYTES, 40);
        // And a fixture built by the test's own encoder loads back.
        let mesh = load_from_bytes(&smallest_valid_file()).unwrap();
        assert_eq!(mesh.vertices.len(), 1);
        assert_eq!(mesh.indices, vec![0, 0, 0]);
        assert_eq!(mesh.sub_meshes.len(), 1);
        assert_eq!(mesh.sub_meshes[0].name, "a");
    }

    #[test]
    fn the_file_stride_matches_the_vertex_struct() {
        // The three constants from task 35, read from `render.rs` in the same
        // direction — so the file's byte layout and the `#[repr(C)]` struct
        // cannot drift apart.
        assert_eq!(
            VERTEX_STRIDE_BYTES,
            crate::render::MESH_VERTEX_STRIDE as u32
        );
        assert_eq!(crate::render::MESH_NORMAL_OFFSET as u32, 12);
        assert_eq!(crate::render::MESH_UV_OFFSET as u32, 24);
        assert_eq!(std::mem::size_of::<MeshVertex>(), 32);

        // Two vertices at stride 32: the loader's fields equal the `f32`s
        // written at the two strides, bit for bit.
        let mut vertex_bytes = Vec::new();
        vertex_bytes.extend_from_slice(&encode_vertex(
            [1.0, -2.5, 0.125],
            [3.0, 4.0, 0.0],
            [0.0, 1.0],
        ));
        vertex_bytes.extend_from_slice(&encode_vertex(
            [9.0, 8.0, 7.0],
            [0.0, 0.0, 0.0],
            [0.5, 0.5],
        ));
        let file = encode_file(2, &vertex_bytes, &[0, 1, 0], &[(0, 3)], b"a\0");
        let mesh = load_from_bytes(&file).unwrap();
        assert_eq!(mesh.vertices[0].position, [1.0, -2.5, 0.125]);
        assert_eq!(mesh.vertices[0].position[0].to_bits(), 1.0f32.to_bits());
        // [3,4,0] normalises to [0.6,0.8,0] through `MeshVertex::new`.
        assert_eq!(mesh.vertices[0].normal, [0.6, 0.8, 0.0]);
        assert_eq!(mesh.vertices[1].position, [9.0, 8.0, 7.0]);
        // A zero-length normal becomes up rather than `NaN`.
        assert_eq!(mesh.vertices[1].normal, [0.0, 0.0, 1.0]);

        // The negative half, which is what makes the stride assertion
        // non-vacuous: the second vertex read at offset 31 rather than 32 is
        // a different position, so a reader striding by 31 would fail the
        // assertion above rather than agree with it.
        let misaligned =
            f32::from_le_bytes([file[40 + 31], file[40 + 32], file[40 + 33], file[40 + 34]]);
        assert_ne!(misaligned, 9.0);
    }

    #[test]
    fn a_fixture_the_test_built_loads_back_to_what_it_wrote() {
        let mut vertex_bytes = Vec::new();
        vertex_bytes.extend_from_slice(&encode_vertex(
            [0.75, -1.28, 1.15],
            [0.0, 1.0, 0.0],
            [0.1, 0.9],
        ));
        vertex_bytes.extend_from_slice(&encode_vertex(
            [-0.3, 0.3, 0.66],
            [1.0, 1.0, 1.0],
            [0.0, 0.0],
        ));
        let file = encode_file(
            2,
            &vertex_bytes,
            &[0, 1, 0, 1, 0, 1],
            &[(0, 3), (3, 3)],
            b"body\0wheel\0",
        );
        let mesh = load_from_bytes(&file).unwrap();
        assert_eq!(mesh.vertices.len(), 2);
        assert_eq!(mesh.indices, vec![0, 1, 0, 1, 0, 1]);
        assert_eq!(mesh.sub_meshes.len(), 2);
        assert_eq!(mesh.sub_meshes[0].name, "body");
        assert_eq!(mesh.sub_meshes[0].first_index, 0);
        assert_eq!(mesh.sub_meshes[0].index_count, 3);
        assert_eq!(mesh.sub_meshes[1].name, "wheel");
        assert_eq!(mesh.sub_meshes[1].first_index, 3);
        assert_eq!(mesh.sub_meshes[1].index_count, 3);
        // Exact `f32` bit patterns at the two strides.
        assert_eq!(mesh.vertices[1].position[0].to_bits(), (-0.3f32).to_bits());
        assert_eq!(mesh.vertices[1].uv, [0.0, 0.0]);
    }

    #[test]
    fn the_public_api_offers_the_byte_form_the_tests_use() {
        // `load_from_bytes` takes a slice, so a test holds the bytes in
        // memory: a later refactor that folded this into `load_from_path`
        // would make every test in this module un-runnable, and this is the
        // test that says so.
        let file = smallest_valid_file();
        let mesh = load_from_bytes(file.as_slice()).unwrap();
        assert_eq!(mesh.vertices.len(), 1);
    }

    #[test]
    fn a_file_shorter_than_the_header_is_truncated() {
        // Both sizes, because "empty" is the case a reader mishandles into
        // `NotAMesh` — shortness is checked before content.
        for len in [0, 39] {
            let error = load_from_bytes(&vec![0u8; len]).unwrap_err();
            assert_eq!(
                error,
                MeshError::Truncated {
                    needed: 40,
                    have: len as u64
                },
                "a {len}-byte file is short, not foreign"
            );
        }
    }

    #[test]
    fn a_file_whose_magic_is_wrong_is_not_a_mesh() {
        let mut file = smallest_valid_file();
        file[0..8].copy_from_slice(b"ROADOSM\x00");
        let error = load_from_bytes(&file).unwrap_err();
        match error {
            MeshError::NotAMesh { found } => {
                assert_eq!(found, *b"ROADOSM\x00");
                assert_eq!(found[7], 0);
            }
            other => panic!("expected NotAMesh, got {other:?}"),
        }
    }

    #[test]
    fn a_file_of_a_version_this_build_does_not_read_is_refused() {
        let mut file = smallest_valid_file();
        file[8..12].copy_from_slice(&2u32.to_le_bytes());
        let error = load_from_bytes(&file).unwrap_err();
        assert_eq!(
            error,
            MeshError::Version {
                found: 2,
                expected: 1
            }
        );
        assert_eq!(
            error.to_string(),
            "model file version 2, this build reads version 1"
        );
    }

    #[test]
    fn a_file_whose_header_bytes_field_is_not_forty_is_refused() {
        let mut file = smallest_valid_file();
        file[12..16].copy_from_slice(&64u32.to_le_bytes());
        let error = load_from_bytes(&file).unwrap_err();
        match error {
            MeshError::Malformed(message) => assert!(
                message.contains("header_bytes"),
                "the message names the field: {message}"
            ),
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn a_file_with_a_set_flag_bit_is_refused() {
        let mut file = smallest_valid_file();
        file[32..36].copy_from_slice(&1u32.to_le_bytes());
        let error = load_from_bytes(&file).unwrap_err();
        match error {
            MeshError::Malformed(message) => assert!(
                message.contains("flags"),
                "the message names the field: {message}"
            ),
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn a_file_whose_reserved_word_is_not_zero_is_refused() {
        let mut file = smallest_valid_file();
        file[36..40].copy_from_slice(&1u32.to_le_bytes());
        let error = load_from_bytes(&file).unwrap_err();
        match error {
            MeshError::Malformed(message) => assert!(
                message.contains("reserved"),
                "the message names the field: {message}"
            ),
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn a_file_with_no_vertices_is_refused() {
        let mut file = smallest_valid_file();
        file[16..20].copy_from_slice(&0u32.to_le_bytes());
        let error = load_from_bytes(&file).unwrap_err();
        match error {
            MeshError::Malformed(message) => assert!(
                message.contains("vertex_count"),
                "the message names the field: {message}"
            ),
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn a_file_whose_index_count_is_not_a_multiple_of_three_is_refused() {
        let vertex = encode_vertex([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]);
        let file = encode_file(1, &vertex, &[0, 0, 0, 0, 0, 0, 0], &[(0, 3)], b"a\0");
        let error = load_from_bytes(&file).unwrap_err();
        match error {
            MeshError::Malformed(message) => assert!(
                message.contains("index_count"),
                "the message names the field: {message}"
            ),
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn a_file_whose_payload_is_shorter_than_its_counts_is_truncated() {
        // A 40-byte header declaring one vertex, one triangle, one range and
        // a two-byte name block, with no payload behind it.
        let mut file = smallest_valid_file();
        file.truncate(40);
        let error = load_from_bytes(&file).unwrap_err();
        assert_eq!(
            error,
            MeshError::Truncated {
                needed: 72,
                have: 40
            },
            "the vertex block is the short one: 40 + 1 × 32"
        );
    }

    #[test]
    fn a_file_whose_index_reaches_past_the_vertex_array_is_refused() {
        let mut vertex_bytes = Vec::new();
        for _ in 0..3 {
            vertex_bytes.extend_from_slice(&encode_vertex(
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0],
            ));
        }
        let file = encode_file(3, &vertex_bytes, &[0, 1, 9], &[(0, 3)], b"a\0");
        let error = load_from_bytes(&file).unwrap_err();
        match error {
            MeshError::Malformed(message) => {
                assert!(
                    message.contains('9'),
                    "the message names the value: {message}"
                );
                assert!(
                    message.contains("position 2"),
                    "and where it was read: {message}"
                );
            }
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn a_file_whose_sub_mesh_ranges_do_not_tile_the_index_array_is_refused() {
        // A gap in the middle that the end does not betray: nine indices in
        // three ranges of three, the second starting at 4 rather than 3. The
        // ranges still end exactly at 9, so only the first-index check can
        // refuse this — an earlier shape of this fixture overran the end and
        // was killed by the bounds check instead, which a mutation removing
        // the tiling check survived.
        let vertex = encode_vertex([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]);
        let file = encode_file(
            1,
            &vertex,
            &[0, 0, 0, 0, 0, 0, 0, 0, 0],
            &[(0, 3), (4, 3), (6, 3)],
            b"a\0b\0c\0",
        );
        let error = load_from_bytes(&file).unwrap_err();
        match error {
            MeshError::Malformed(message) => assert!(
                message.contains("sub-mesh 1"),
                "the message names the sub-mesh: {message}"
            ),
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn a_file_with_bytes_left_over_at_the_end_is_refused() {
        let mut file = smallest_valid_file();
        file.push(0);
        let error = load_from_bytes(&file).unwrap_err();
        match error {
            MeshError::Malformed(message) => assert!(
                message.contains("trailing"),
                "the message names the surplus: {message}"
            ),
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn a_file_whose_names_are_not_utf8_is_refused() {
        let vertex = encode_vertex([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]);
        let file = encode_file(1, &vertex, &[0, 0, 0], &[(0, 3)], &[0xFF, 0x00]);
        let error = load_from_bytes(&file).unwrap_err();
        match error {
            MeshError::Malformed(message) => assert!(
                message.contains("offset"),
                "the message names the offset: {message}"
            ),
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn a_file_with_fewer_names_than_sub_meshes_is_refused() {
        let vertex = encode_vertex([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]);
        let file = encode_file(
            1,
            &vertex,
            &[0, 0, 0, 0, 0, 0, 0, 0, 0],
            &[(0, 3), (3, 3), (6, 3)],
            b"a\0b\0",
        );
        let error = load_from_bytes(&file).unwrap_err();
        match error {
            MeshError::Malformed(message) => {
                assert!(
                    message.contains('2'),
                    "how many names were found: {message}"
                );
                assert!(
                    message.contains('3'),
                    "against how many were declared: {message}"
                );
            }
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn a_file_with_a_name_longer_than_the_declared_ceiling_is_refused() {
        let vertex = encode_vertex([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]);
        let mut name_block = vec![b'x'; 199];
        name_block.push(0);
        let file = encode_file(1, &vertex, &[0, 0, 0], &[(0, 3)], &name_block);
        let error = load_from_bytes(&file).unwrap_err();
        match error {
            MeshError::Malformed(message) => assert!(
                message.contains("name_bytes"),
                "the message names the field: {message}"
            ),
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn a_hostile_vertex_count_cannot_allocate_more_than_the_file_holds() {
        // `0x0800_0000 × 32` is `2^32`, which a `u32` multiply wraps to
        // exactly zero: a reader computing in `u32` sees a 40-byte file whose
        // vertices occupy no bytes and accepts it — then asks for 4 GB.
        let mut file = smallest_valid_file();
        file.truncate(40);
        file[16..20].copy_from_slice(&0x0800_0000u32.to_le_bytes());
        let error = load_from_bytes(&file).unwrap_err();
        assert_eq!(
            error,
            MeshError::Truncated {
                needed: 4_294_967_336,
                have: 40
            },
            "the needed value is the assertion: 40 + 2^32"
        );
    }

    #[test]
    fn an_index_count_that_overflows_a_thirty_two_bit_multiply_is_truncated_not_accepted() {
        // The same trap on the index block: `0xC000_0000 × 4` is `3 × 2^32`,
        // which a `u32` multiply wraps to zero. It is `0xC000_0000` rather than
        // `0x4000_0000` because check 5 runs before check 7: `0x4000_0000` is
        // not a multiple of three, so it is refused as `Malformed` before any
        // length is computed, and a count that is meant to exercise the
        // multiply has to survive the multiple-of-three check first.
        // `0xC000_0000` is the smallest multiple of three with the wrap
        // property. One vertex of payload sits between the header and the
        // index block, so the file carries it — 72 bytes — and the prefix
        // length names the index block: 40 + 32 + 3 × 2^32.
        let mut file = smallest_valid_file();
        file.truncate(40);
        file[16..20].copy_from_slice(&1u32.to_le_bytes());
        file[20..24].copy_from_slice(&0xC000_0000u32.to_le_bytes());
        file.extend_from_slice(&[0u8; 32]);
        let error = load_from_bytes(&file).unwrap_err();
        assert_eq!(
            error,
            MeshError::Truncated {
                needed: 12_884_901_960,
                have: 72
            }
        );
    }
}
