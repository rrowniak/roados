//! Mesh geometry: vertex format, CPU-side mesh data, and the store that holds
//! uploaded meshes.
//!
//! A mesh is one vertex buffer and one index buffer. The model's named parts
//! (e.g. `body`, `wheel-front-left`) are `(first_index, index_count)` ranges
//! into that shared index buffer — this is what lets each part be drawn with its
//! own transform (tasks 36, 37, 40). The alternative — one VBO and IBO per
//! sub-mesh — would mean five uploads instead of one and five attribute setups
//! for a vertex format that is identical across all five, in exchange for the
//! ability to run out of a buffer independently, which a car never does.
//!
//! **Rejected alternative: per-sub-mesh buffers.** Five separate VBO/IBO pairs
//! would mean five buffer uploads, five VAO configurations, and five draw calls
//! with no ability to share vertex data between parts. The shared-buffer design
//! enables a single upload, one VAO, and per-part transforms via `glDrawElements`
//! with byte offsets — the whole point of the sub-mesh design.
//!
//! **Rejected alternative: a non-indexed mesh.** `draw_arrays` would require
//! duplicating every shared vertex (a car body shares vertices across hundreds
//! of triangles). The index buffer is what makes a mesh a mesh: it turns
//! O(triangles) vertex data into O(unique vertices). The one `draw_arrays` in
//! the crate is the fullscreen blur quad, which has no shared vertices.
//!
//! **Rejected alternative: `quad_indices` reuse.** `quad_indices` generates
//! indices for a quad (two triangles, four vertices, six indices). A mesh's
//! topology is arbitrary — vertices are shared between triangles in ways a quad
//! never is — so its index buffer is whatever the model says it is. The
//! function's doc comment says it best: *"A quad is four vertices, so the
//! indices only change when the vertex count per quad does."* That is false for
//! a mesh.
//!
//! The GL objects (VAO, VBO, IBO) live on [`crate::render::Renderer`], where the
//! crate's other GL objects live. The CPU geometry lives here, in plain data
//! structures with no GL in them, so the arithmetic that decides whether a mesh
//! is valid — range containment, index bounds, byte offsets — is testable on a
//! machine with no display.
//!
//! **No eviction.** The texture cache evicts because a 2048² atlas has a hard
//! pixel budget and many small images contend for it, so its unit is a *shelf*
//! and LRU is the only policy that fits; a mesh has no shelf, no budget and,
//! in this project, one resident model drawn every frame, so an LRU over it is a
//! data structure with one element and no event that could ever fire.
//! Re-uploading an evicted car every frame is a guaranteed stall, not a saving.
//! `Renderer` needs no drop either: `ShadowTarget::drop`'s own doc comment
//! already records that *"no GL object is deleted here, and that is the same
//! decision `crate::render` makes"* — the buffers live until `Context` tears the
//! GL context down. `Drop` on a new mesh type that deletes three GL objects
//! while its three siblings do not would be a distinction without a difference.

use crate::render::RenderError;

/// One vertex of a triangle mesh: object-space position, normal, and texture
/// coordinate.
///
/// The layout matches the attribute pointers the mesh VAO records (see
/// [`crate::render::Renderer::new`] and the `MESH_VERTEX_STRIDE` /
/// `MESH_NORMAL_OFFSET` / `MESH_UV_OFFSET` constants in `render.rs`):
///
/// | location | attribute   | components | offset |
/// |----------|-------------|------------|--------|
/// | 0        | `a_position` | 3, `GL_FLOAT` | 0      |
/// | 1        | `a_normal`   | 3, `GL_FLOAT` | 12     |
/// | 2        | `a_uv`       | 2, `GL_FLOAT` | 24     ///
///
/// Locations 0, 1 and 2 are reused from the other three VAOs deliberately. It
/// is safe because the enabled-array and pointer state belongs to the VAO, not
/// to the context, and each VAO binds its own buffers — which is the same
/// reason `text_vao` and `image_vao` already reuse 0, 1 and 2 with different
/// meanings.
///
/// **No colour field.** One material and one texture cover all five meshes, so
/// a per-vertex colour would be 16 bytes of constant data per vertex — a 50 %
/// increase in vertex bandwidth to carry a value the shader reads once from a
/// uniform. **No padding.** All three fields are `f32`, so `repr(C)` puts every
/// field at a multiple of 4 and the struct's size is already 8 × 4 = **32
/// bytes**; there is no alignment requirement to satisfy, and GLES 3.1 imposes
/// none on buffer data.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshVertex {
    /// Position in object space, right-handed, metres.
    pub position: [f32; 3],
    /// Normal in object space, **unit length**.
    ///
    /// Normalised by [`MeshVertex::new`]. A zero-length normal becomes
    /// `[0.0, 0.0, 1.0]` rather than `NaN`, because one `NaN` in a vertex
    /// buffer makes the triangle it belongs to disappear with no GL error.
    pub normal: [f32; 3],
    /// Texture coordinate into the material's texture, `(0,0)` at its bottom
    /// left.
    pub uv: [f32; 2],
}

impl MeshVertex {
    /// Creates a vertex with a **normalised** normal.
    ///
    /// A zero-length normal becomes `[0.0, 0.0, 1.0]` rather than `NaN`,
    /// because one `NaN` in a vertex buffer makes the triangle it belongs to
    /// disappear with no GL error. Both branches are unit-tested.
    pub fn new(position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        let normal = {
            let len =
                (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            if len > 0.0 {
                [normal[0] / len, normal[1] / len, normal[2] / len]
            } else {
                [0.0, 0.0, 1.0]
            }
        };
        MeshVertex {
            position,
            normal,
            uv,
        }
    }
}

/// One sub-mesh's range in the shared index buffer.
///
/// A newtype rather than two `u32` fields on the command: the two cannot be
/// transposed by accident, and `first_index: 1` is **4 bytes**, not 1 — the
/// conversion [`SubMeshRange::byte_offset`] is the single place that knows it.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SubMeshRange {
    /// First index of the range, absolute into `Mesh::indices`.
    pub first_index: u32,
    /// How many indices the range holds; a multiple of three.
    pub index_count: u32,
}

impl SubMeshRange {
    /// Converts `first_index` to the byte offset GL's `draw_elements` expects.
    ///
    /// The conversion is `first_index × size_of::<u32>()`, checked, returning
    /// [`RenderError::Gl`](crate::render::RenderError::Gl) rather than
    /// wrapping. A range past the end of the index buffer is what
    /// `draw_elements` reads, and without robust buffer access that is
    /// undefined geometry rather than an error — so the draw site
    /// bounds-checks first and this conversion only refuses an offset no
    /// address can hold.
    pub fn byte_offset(&self) -> Result<i32, RenderError> {
        let byte_offset = self
            .first_index
            .checked_mul(std::mem::size_of::<u32>() as u32)
            .ok_or_else(|| {
                RenderError::Gl("sub-mesh byte offset exceeds the u32 range".to_string())
            })?;
        i32::try_from(byte_offset)
            .map_err(|_| RenderError::Gl("sub-mesh byte offset exceeds the i32 range".to_string()))
    }
}

/// A named index range into the shared index buffer.
///
/// The whole model is one vertex buffer and one index buffer, and the five
/// named meshes are `(first_index, index_count)` ranges into that shared index
/// buffer. `first_index` is an **index position**, not a byte offset.
#[derive(Clone, Debug, PartialEq)]
pub struct SubMesh {
    /// The sub-mesh's name, read from the model file (task 38).
    pub name: String,
    /// The first index in the shared index buffer.
    pub first_index: u32,
    /// The number of indices in this sub-mesh.
    pub index_count: u32,
}

impl SubMesh {
    /// Returns this sub-mesh's range in the shared index buffer.
    #[must_use]
    pub fn range(&self) -> SubMeshRange {
        SubMeshRange {
            first_index: self.first_index,
            index_count: self.index_count,
        }
    }
}

/// A complete mesh: vertices, indices, and named sub-mesh ranges.
///
/// `indices` are **absolute vertex indices into `vertices`**, already offset by
/// the loader; there is no `vertex_base` field to add at draw time, because
/// rebasing would need either a per-draw integer uniform or a CPU rewrite of
/// the whole index buffer, and a loader that offsets once is cheaper than
/// either.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    /// All vertices of the model.
    pub vertices: Vec<MeshVertex>,
    /// All indices of the model, referencing `vertices`.
    pub indices: Vec<u32>,
    /// Named sub-mesh ranges into `indices`.
    pub sub_meshes: Vec<SubMesh>,
}

/// A handle to an uploaded mesh in the renderer's store.
///
/// Indexes `MeshStore`'s slots and appears in task 37's `DrawCommand::Mesh`.
/// `pub(crate)` for the store, `pub` for the three data types, so the API
/// surface is what tasks 37 and 38 need and nothing more.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MeshId(pub(crate) u32);

impl MeshId {
    /// Creates a mesh handle from a raw store slot.
    ///
    /// The slot is what the store's `push` returns, and a handle for a slot
    /// the store does not hold draws nothing: the draw site skips an unknown
    /// id rather than panicking, so a handle is never a promise the mesh
    /// exists. The constructor is public so a caller can name the handle a
    /// recording needs — including [`Painter::mesh`](crate::paint::Painter::mesh)'s
    /// doc example, which records without opening a window.
    #[must_use]
    pub fn new(id: u32) -> Self {
        MeshId(id)
    }
}

/// CPU-side record of an uploaded mesh.
///
/// The renderer appends to `meshes` and every mesh lives until the renderer
/// drops. No cache, no eviction, no pin.
#[derive(Clone, Debug)]
pub(crate) struct MeshRecord {
    mesh: Mesh,
    #[expect(dead_code)]
    vertex_base: u32,
    #[expect(dead_code)]
    index_base: u32,
}

impl MeshRecord {
    /// How many indices the stored mesh holds.
    ///
    /// The draw site bounds-checks a command's range against this before
    /// issuing the draw: a range past the end of the index buffer is what
    /// `draw_elements` reads, and without robust buffer access that is
    /// undefined geometry rather than an error.
    pub(crate) fn indices_len(&self) -> usize {
        self.mesh.indices.len()
    }
}

/// The store that owns all uploaded meshes' CPU geometry.
///
/// `Renderer::upload_mesh` appends and every mesh lives until the renderer
/// drops. No cache, no eviction, no pin.
pub(crate) struct MeshStore {
    meshes: Vec<MeshRecord>,
}

impl MeshStore {
    /// Creates an empty mesh store.
    pub(crate) fn new() -> Self {
        MeshStore { meshes: Vec::new() }
    }

    /// Returns the mesh for `id`, or `None` if the id is unknown.
    #[allow(dead_code)]
    pub(crate) fn get(&self, id: MeshId) -> Option<&MeshRecord> {
        self.meshes.get(id.0 as usize)
    }

    /// Appends a mesh and returns its `MeshId`.
    ///
    /// The bases are the current buffer lengths in vertices and indices.
    pub(crate) fn push(&mut self, mesh: Mesh, vertex_base: u32, index_base: u32) -> MeshId {
        let id = MeshId(self.meshes.len() as u32);
        self.meshes.push(MeshRecord {
            mesh,
            vertex_base,
            index_base,
        });
        id
    }

    /// Returns the total number of vertices across all stored meshes.
    pub(crate) fn total_vertices(&self) -> u32 {
        self.meshes
            .iter()
            .map(|m| m.mesh.vertices.len() as u32)
            .sum()
    }

    /// Returns the total number of indices across all stored meshes.
    pub(crate) fn total_indices(&self) -> u32 {
        self.meshes
            .iter()
            .map(|m| m.mesh.indices.len() as u32)
            .sum()
    }
}

/// Converts a sub-mesh's `first_index` to the byte offset GL's
/// `draw_elements` expects.
///
/// A one-line delegate to [`SubMeshRange::byte_offset`]: a duplicate with no
/// test is two numbers that can drift, so a test asserts the two return the
/// same value for a fixture of ranges.
pub fn sub_mesh_byte_offset(sub: &SubMesh) -> Result<i32, RenderError> {
    sub.range().byte_offset()
}

/// Validates that all indices in `mesh` fall within the slot's vertex range
/// `[vertex_base, vertex_base + mesh.vertices.len())`. This is a pure
/// function so it can be tested without a display. The loader produces
/// indices already offset by the concatenation order; this check ensures
/// the offsetting agrees with the store's layout.
pub fn validate_mesh_indices_against_base(
    mesh: &Mesh,
    vertex_base: u32,
) -> Result<(), RenderError> {
    let vertex_end = vertex_base + mesh.vertices.len() as u32;
    for (i, &idx) in mesh.indices.iter().enumerate() {
        if idx < vertex_base || idx >= vertex_end {
            return Err(RenderError::Gl(format!(
                "index {} at position {} not in slot range [{}, {})",
                idx, i, vertex_base, vertex_end
            )));
        }
    }
    Ok(())
}

/// Validates a mesh before upload, returning an error if invalid.
///
/// This is a pure function with no GL dependencies, so it can be tested
/// without a display. It checks:
/// - vertices not empty
/// - indices not empty
/// - sub_meshes not empty
/// - every sub-mesh range is within indices bounds
/// - every index value is within vertices bounds
pub fn validate_mesh(mesh: &Mesh) -> Result<(), RenderError> {
    if mesh.vertices.is_empty() {
        return Err(RenderError::Gl("mesh vertices empty".to_string()));
    }
    if mesh.indices.is_empty() {
        return Err(RenderError::Gl("mesh indices empty".to_string()));
    }
    if mesh.sub_meshes.is_empty() {
        return Err(RenderError::Gl("mesh sub-meshes empty".to_string()));
    }

    for sub in &mesh.sub_meshes {
        let end = sub
            .first_index
            .checked_add(sub.index_count)
            .ok_or_else(|| {
                RenderError::Gl(format!(
                    "sub-mesh {}: first_index + index_count overflow",
                    sub.name
                ))
            })?;
        if end > mesh.indices.len() as u32 {
            return Err(RenderError::Gl(format!(
                "sub-mesh {}: index range exceeds indices.len()",
                sub.name
            )));
        }
    }

    for (i, &idx) in mesh.indices.iter().enumerate() {
        if idx >= mesh.vertices.len() as u32 {
            return Err(RenderError::Gl(format!(
                "index {} at position {} >= vertices.len() {}",
                idx,
                i,
                mesh.vertices.len()
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_vertex_size_and_offsets() {
        // Stride: 8 f32 = 32 bytes, no padding. Assert against the constants
        // in render.rs so a change to the constant without a matching struct
        // change is caught.
        assert_eq!(
            std::mem::size_of::<MeshVertex>(),
            crate::render::MESH_VERTEX_STRIDE as usize
        );
        // Offsets via std::mem::offset_of! (stable since Rust 1.77, floor is 1.85).
        assert_eq!(
            std::mem::offset_of!(MeshVertex, normal),
            crate::render::MESH_NORMAL_OFFSET as usize
        );
        assert_eq!(
            std::mem::offset_of!(MeshVertex, uv),
            crate::render::MESH_UV_OFFSET as usize
        );
    }

    #[test]
    fn mesh_vertex_new_normalises() {
        let v = MeshVertex::new([1.0, 2.0, 3.0], [3.0, 4.0, 0.0], [0.5, 0.5]);
        let len =
            (v.normal[0] * v.normal[0] + v.normal[1] * v.normal[1] + v.normal[2] * v.normal[2])
                .sqrt();
        assert!(
            (len - 1.0).abs() < 1e-6,
            "normal should be unit length, got {}",
            len
        );
        assert_eq!(v.position, [1.0, 2.0, 3.0]);
        assert_eq!(v.uv, [0.5, 0.5]);
    }

    #[test]
    fn mesh_vertex_new_zero_normal_becomes_up() {
        let v = MeshVertex::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0]);
        assert_eq!(v.normal, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn sub_mesh_byte_offset_conversion() {
        // first_index: 1 → 4 bytes (size_of::<u32>() == 4)
        let sub = SubMesh {
            name: "test".to_string(),
            first_index: 1,
            index_count: 6,
        };
        assert_eq!(sub_mesh_byte_offset(&sub).unwrap(), 4);

        // first_index: 0 → 0
        let sub = SubMesh {
            name: "test".to_string(),
            first_index: 0,
            index_count: 6,
        };
        assert_eq!(sub_mesh_byte_offset(&sub).unwrap(), 0);

        // Large index that would overflow i32 when multiplied by 4
        // i32::MAX = 2_147_483_647, so i32::MAX / 4 = 536_870_911
        // u32::MAX = 4_294_967_295, so we need first_index > i32::MAX / 4
        let sub = SubMesh {
            name: "test".to_string(),
            first_index: (i32::MAX as u32 / 4) + 1,
            index_count: 6,
        };
        let result = sub_mesh_byte_offset(&sub);
        assert!(result.is_err(), "offset past i32::MAX should return Err");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceeds the i32 range"));
    }

    #[test]
    fn sub_mesh_byte_offset_agrees_with_range_byte_offset() {
        // `sub_mesh_byte_offset` is a one-line delegate to
        // `SubMeshRange::byte_offset`: a duplicate with no test is two numbers
        // that can drift, so both are asserted to return the same value for a
        // fixture of ranges — including `first_index: 1`, which is **4 bytes**,
        // not 1, and an offset past `i32`, which is `Err` on both.
        for first_index in [0, 1, 7, 1_000_000, (i32::MAX as u32 / 4) + 1] {
            let sub = SubMesh {
                name: "test".to_string(),
                first_index,
                index_count: 6,
            };
            assert_eq!(
                sub_mesh_byte_offset(&sub).map_err(|e| e.to_string()),
                sub.range().byte_offset().map_err(|e| e.to_string()),
                "delegate and method disagree at first_index {first_index}"
            );
        }
        let one = SubMesh {
            name: "test".to_string(),
            first_index: 1,
            index_count: 3,
        };
        assert_eq!(one.range().byte_offset().unwrap(), 4);
    }

    #[test]
    fn five_mesh_fixture_contiguous_non_overlapping() {
        // A fixture with five SubMesh ranges asserting they are contiguous,
        // non-overlapping, cover indices.len() exactly, and every index value
        // is < vertices.len(). The five named meshes match the car model:
        // body, wheel-front-left, wheel-front-right, wheel-back-left, wheel-back-right.
        let vertices = vec![
            MeshVertex::new([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
            MeshVertex::new([1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0]),
            MeshVertex::new([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0]),
            MeshVertex::new([1.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0]),
            MeshVertex::new([2.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
            MeshVertex::new([3.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0]),
            MeshVertex::new([2.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0]),
            MeshVertex::new([3.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0]),
            MeshVertex::new([4.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
            MeshVertex::new([5.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0]),
            MeshVertex::new([4.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0]),
            MeshVertex::new([5.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0]),
            MeshVertex::new([6.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
            MeshVertex::new([7.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0]),
            MeshVertex::new([6.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0]),
            MeshVertex::new([7.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0]),
            MeshVertex::new([8.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
            MeshVertex::new([9.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0]),
            MeshVertex::new([8.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0]),
            MeshVertex::new([9.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0]),
        ];
        // 5 quads = 5 * 6 = 30 indices
        let indices = vec![
            0, 1, 2, 2, 1, 3, // body (quad 0)
            4, 5, 6, 6, 5, 7, // wheel-front-left (quad 1)
            8, 9, 10, 10, 9, 11, // wheel-front-right (quad 2)
            12, 13, 14, 14, 13, 15, // wheel-back-left (quad 3)
            16, 17, 18, 18, 17, 19, // wheel-back-right (quad 4)
        ];
        let sub_meshes = vec![
            SubMesh {
                name: "body".to_string(),
                first_index: 0,
                index_count: 6,
            },
            SubMesh {
                name: "wheel-front-left".to_string(),
                first_index: 6,
                index_count: 6,
            },
            SubMesh {
                name: "wheel-front-right".to_string(),
                first_index: 12,
                index_count: 6,
            },
            SubMesh {
                name: "wheel-back-left".to_string(),
                first_index: 18,
                index_count: 6,
            },
            SubMesh {
                name: "wheel-back-right".to_string(),
                first_index: 24,
                index_count: 6,
            },
        ];
        let mesh = Mesh {
            vertices: vertices.clone(),
            indices: indices.clone(),
            sub_meshes,
        };

        // Assert sub-meshes are contiguous and cover exactly indices.len()
        let mut total = 0;
        let mut prev_end = 0;
        for sub in &mesh.sub_meshes {
            assert!(
                sub.first_index + sub.index_count <= mesh.indices.len() as u32,
                "sub-mesh {} out of bounds",
                sub.name
            );
            // Assert contiguous (no gaps)
            assert_eq!(
                sub.first_index, prev_end,
                "sub-mesh {} not contiguous with previous",
                sub.name
            );
            // Check all index values are in range
            for idx in mesh.indices
                [sub.first_index as usize..(sub.first_index + sub.index_count) as usize]
                .iter()
            {
                assert!(
                    *idx < mesh.vertices.len() as u32,
                    "index {} >= vertices.len() {}",
                    idx,
                    mesh.vertices.len()
                );
            }
            total += sub.index_count;
            prev_end = sub.first_index + sub.index_count;
        }
        assert_eq!(
            total as usize,
            mesh.indices.len(),
            "sub-meshes must cover indices exactly"
        );
        // Also assert non-overlapping implicitly by contiguity + exact cover
    }

    #[test]
    fn mesh_store_push_and_get() {
        let mut store = MeshStore::new();
        let mesh = Mesh {
            vertices: vec![MeshVertex::new(
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0],
            )],
            indices: vec![0],
            sub_meshes: vec![SubMesh {
                name: "test".to_string(),
                first_index: 0,
                index_count: 1,
            }],
        };
        let id = store.push(mesh, 0, 0);
        assert_eq!(id.0, 0);
        let record = store.get(id).unwrap();
        assert_eq!(record.mesh.sub_meshes[0].name, "test");
        assert!(store.get(MeshId(999)).is_none());
    }

    #[test]
    fn validate_mesh_empty_vertices() {
        let mesh = Mesh {
            vertices: vec![],
            indices: vec![0],
            sub_meshes: vec![SubMesh {
                name: "test".to_string(),
                first_index: 0,
                index_count: 1,
            }],
        };
        let err = validate_mesh(&mesh).unwrap_err();
        assert!(matches!(err, RenderError::Gl(msg) if msg.contains("vertices empty")));
    }

    #[test]
    fn validate_mesh_empty_indices() {
        let mesh = Mesh {
            vertices: vec![MeshVertex::new(
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0],
            )],
            indices: vec![],
            sub_meshes: vec![SubMesh {
                name: "test".to_string(),
                first_index: 0,
                index_count: 1,
            }],
        };
        let err = validate_mesh(&mesh).unwrap_err();
        assert!(matches!(err, RenderError::Gl(msg) if msg.contains("indices empty")));
    }

    #[test]
    fn validate_mesh_empty_sub_meshes() {
        let mesh = Mesh {
            vertices: vec![MeshVertex::new(
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0],
            )],
            indices: vec![0],
            sub_meshes: vec![],
        };
        let err = validate_mesh(&mesh).unwrap_err();
        assert!(matches!(err, RenderError::Gl(msg) if msg.contains("sub-meshes empty")));
    }

    #[test]
    fn validate_mesh_sub_mesh_out_of_range() {
        let mesh = Mesh {
            vertices: vec![MeshVertex::new(
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0],
            )],
            indices: vec![0],
            sub_meshes: vec![SubMesh {
                name: "test".to_string(),
                first_index: 1,
                index_count: 1,
            }],
        };
        let err = validate_mesh(&mesh).unwrap_err();
        assert!(matches!(err, RenderError::Gl(msg) if msg.contains("exceeds indices.len()")));
    }

    #[test]
    fn validate_mesh_index_out_of_range() {
        let mesh = Mesh {
            vertices: vec![MeshVertex::new(
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0],
            )],
            indices: vec![1], // Index 1 >= vertices.len() (1)
            sub_meshes: vec![SubMesh {
                name: "test".to_string(),
                first_index: 0,
                index_count: 1,
            }],
        };
        let err = validate_mesh(&mesh).unwrap_err();
        assert!(matches!(err, RenderError::Gl(msg) if msg.contains(">= vertices.len()")));
    }

    #[test]
    fn validate_mesh_valid() {
        let mesh = Mesh {
            vertices: vec![
                MeshVertex::new([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
                MeshVertex::new([1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0]),
            ],
            indices: vec![0, 1],
            sub_meshes: vec![SubMesh {
                name: "test".to_string(),
                first_index: 0,
                index_count: 2,
            }],
        };
        validate_mesh(&mesh).unwrap();
    }

    #[test]
    fn validate_mesh_indices_against_base_valid() {
        // Indices [10, 11] with vertex_base=10, vertices.len()=2 -> range [10, 12)
        let mesh = Mesh {
            vertices: vec![
                MeshVertex::new([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
                MeshVertex::new([1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0]),
            ],
            indices: vec![10, 11],
            sub_meshes: vec![SubMesh {
                name: "test".to_string(),
                first_index: 0,
                index_count: 2,
            }],
        };
        validate_mesh_indices_against_base(&mesh, 10).unwrap();
    }

    #[test]
    fn validate_mesh_indices_against_base_below() {
        // Index 9 is below vertex_base=10
        let mesh = Mesh {
            vertices: vec![
                MeshVertex::new([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
                MeshVertex::new([1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0]),
            ],
            indices: vec![9, 11],
            sub_meshes: vec![SubMesh {
                name: "test".to_string(),
                first_index: 0,
                index_count: 2,
            }],
        };
        let err = validate_mesh_indices_against_base(&mesh, 10).unwrap_err();
        assert!(matches!(err, RenderError::Gl(ref msg) if msg.contains("not in slot range")));
        assert!(matches!(err, RenderError::Gl(ref msg) if msg.contains("9 at position 0")));
    }

    #[test]
    fn validate_mesh_indices_against_base_above() {
        // Index 12 is >= vertex_base + vertices.len() = 10 + 2 = 12
        let mesh = Mesh {
            vertices: vec![
                MeshVertex::new([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
                MeshVertex::new([1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0]),
            ],
            indices: vec![10, 12],
            sub_meshes: vec![SubMesh {
                name: "test".to_string(),
                first_index: 0,
                index_count: 2,
            }],
        };
        let err = validate_mesh_indices_against_base(&mesh, 10).unwrap_err();
        assert!(matches!(err, RenderError::Gl(ref msg) if msg.contains("not in slot range")));
        assert!(matches!(err, RenderError::Gl(ref msg) if msg.contains("12 at position 1")));
    }
}
