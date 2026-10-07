//! A 4x4 matrix for the mesh pass, and the bridge from `Transform` to it.
//!
//! This module is maths and nothing else: no GL call, no buffer, no program.
//! The mesh **program** does not exist until task 37, so no matrix constructed
//! here reaches a GL context — there is no `uniform_matrix_4_f32_slice` call in
//! this crate yet, and a uniform uploaded to a program whose shader ignores it
//! would be stripped by the GLSL compiler with `get_uniform_location`
//! returning `None`, which is why this task reserves the name (`MESH_MVP_UNIFORM`
//! in `render.rs`) without querying it.
//!
//! ## Column-major, and why
//!
//! `Mat4` stores sixteen `f32` in **column-major** order: `cols[c * 4 + r]` is
//! column `c`, row `r`. GLSL's `mat4` is stored column-major, and
//! `glUniformMatrix4fv` with `transpose = GL_FALSE` takes its sixteen floats in
//! that order, so the array is uploaded **verbatim**. A row-major array would
//! need the transpose flag to mean what it says, and a flag set `true` out of
//! habit — the natural reading for anyone coming from a row-major maths
//! library — silently transposes the matrix with no GL error and a
//! mirrored scene. Keeping the storage column-major removes the flag from the
//! list of things that can be wrong.
//!
//! ## One pre-multiplied `u_mvp`, not three uniforms
//!
//! The mesh pass uploads a single `projection * view * model` matrix as
//! `u_mvp`. The alternative — `u_projection`, `u_view`, `u_model` set
//! separately — costs two extra uploads per draw and one extra order the
//! shader can get wrong: `M * V * P` and `V * P * M` both compile, both run,
//! and both draw a plausible-looking wrong car, and no test in this crate
//! compiles GLSL to catch it. One matrix has exactly one order, written in one
//! Rust expression (`projection.multiply(&view).multiply(&model)`), and that
//! expression is unit-tested here with no display.
//!
//! What one matrix costs is real and is recorded so task 37 inherits the fact
//! instead of rediscovering it: the mesh **fragment** shader needs the
//! model-to-world normal transform separately, and `mat3(u_mvp)` is not one —
//! the projection scales x by `1 / aspect` and z by the near/far terms, so
//! `P * R * n` normalised is not `R * n`. If a caller ever scales
//! non-uniformly per axis, no upper-left 3x3 of an already-uploaded matrix is
//! a valid normal transform, and task 37 must add `u_model` or `u_normal`
//! **beside** `u_mvp` rather than replacing it. Until then there is one
//! caller-shaped hole and zero callers, so there is one uniform.
//!
//! Recomposing `P * V` per sub-mesh costs 128 multiply-adds per draw, 640 per
//! frame at five sub-meshes — nothing at this scale. If a future frame rate
//! ever asks for it, the change is to cache `P * V` on `Renderer`; deciding so
//! now would be optimising a cost nobody measured.
//!
//! ## `Transform` is unchanged, and the bridge lives here
//!
//! `Transform` (`crate::property::Transform`) keeps its five fields, its
//! `identity()`, its `Default`, and its field-by-field `Interpolate`: growing
//! it would break every struct literal (it is a `pub struct` with public
//! fields) and would edit animation semantics — interpolating three Euler
//! angles component-wise does not follow the rotation between two
//! orientations — inside a task whose scope is matrix maths. The conversion
//! `Transform::to_matrix` is declared **in this module, not in `property.rs`**,
//! so the crate's lowest layer gains no dependency on `render`.
//!
//! `Transform.rotation` is a plane rotation, so its axis is **z** even though
//! no doc comment in the crate says so: a 2D rotation has no other axis to be
//! about. The crate's window space is **y-down** (`Rect.y` is an offset from
//! the top edge), so a positive rotation turns **clockwise on screen without a
//! flip and counter-clockwise with the four 2D shaders' `-clip.y`**. Neither
//! direction is behaviour the crate has today — no 2D transform is drawn
//! anywhere — so the choice is task 37's and 40's to make, and the y-flip is
//! deliberately **not** baked into the matrix: all four existing shaders apply
//! it themselves, and a matrix carrying it would be wrong for every one of them.
//!
//! ## The projections, and the near plane's sign
//!
//! Both projections are exposed. The orthographic one is not optional: the
//! mapping the four 2D shaders already perform — `a_pos / u_resolution`, then
//! `* 2.0 - 1.0` — **is** `orthographic(0, w, 0, h, -1, 1)`, and a test pins
//! that rather than asserting it in prose. The near plane is negative, and that
//! is load-bearing: passing the positive, obvious-looking `near = 0.1`,
//! `far = 100.0` maps `z = 0` to `z' = -1.002` — the near plane — instead of
//! `0.0`, which is the value task 34's `0.5 < 0.5` depth arithmetic is written
//! against. A test asserts the trap rather than leaving it as a warning.
//!
//! Perspective produces **OpenGL** clip space, `z` in `[-1, 1]`, because GLES
//! 3.1's clip volume is `[-1, 1]` and it is that which makes window depth
//! `0.5` for `z_ndc = 0.0`. The `[0, 1]` D3D/Vulkan form is the mistake this
//! convention exists to prevent, and the near/far test fails under it.
//!
//! The decided defaults live here so task 37 does not re-decide them:
//! **`fov_y = PI / 4` (45 degrees), `near = 0.1` m, `far = 20.0` m, a ratio of
//! 200** — squarely in the range of hundreds task 34's `DEPTH_BITS` doc
//! comment promises ("far more precision than the raster can show with a
//! near/far ratio task 36 will choose in the range of hundreds"). 45 degrees
//! is the mild end: the vertical half-extent at distance `d` is
//! `d * tan(PI / 8)`, about `0.414 * d`, so at 6 m the frustum is about 5 m
//! tall against a body roughly 1.4 m tall — a visible convergence without
//! wide-angle distortion. 20 m is where the car is off the panel anyway.
//! **Task 40 rotates the camera and must not move `near` or `far`.**
//!
//! ## Degenerate inputs are `None`, not a panic and not infinity
//!
//! Both constructors return `Option<Mat4>`: `None` on every violated
//! precondition. A `perspective` with `fov_y = 0.0` would otherwise divide by
//! `tan(0.0) = 0.0`, which in Rust is `inf` rather than a panic — worse, not
//! better: the matrix uploads, every triangle vanishes, and there is no GL
//! error. `Option` and not `Result` because there is no error to describe and
//! no I/O: a violated precondition is a value the caller does not want. There
//! is deliberately no assertion macro and no debug-only check in this module:
//! a check compiled into the test build but not into the release build would
//! mean the preconditions the suite exercises are not the ones the demo's
//! release binary runs under.
//!
//! `transform_point` returns `None` only when `w` is zero or non-finite. A
//! point at the camera plane has `w = 0` and dividing by it is a division by
//! zero. A point **behind** the camera has `w < 0` and returns `Some` — that is
//! not an error, it is clipping: GL discards a primitive whose clip
//! coordinates fail `-w <= x, y, z <= w`, and a caller returning `None` there
//! would drop a vertex GL would have handled. Returning `None` for `w < 0`
//! reads as defensive and is a defect, which is why the doc comment names it.

use core::f32::consts::PI;

use crate::property::Transform;

/// A 4x4 matrix in column-major order: `cols[c * 4 + r]` is column `c`, row `r`.
///
/// Sixteen `f32`, no padding, 64 bytes. The field is private and `as_slice`
/// is the one way out: it coerces to `&[f32]` at the upload call site with no
/// copy, and the column-major order is what `glUniformMatrix4fv` with
/// `transpose = GL_FALSE` takes verbatim, so no transpose flag can silently
/// swap it. There is deliberately no second accessor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4 {
    /// The sixteen floats, column-major.
    cols: [f32; 16],
}

impl Mat4 {
    /// The identity matrix.
    ///
    /// A function and not an associated constant, because
    /// `Transform::identity()` in `property.rs` is one and the nearest
    /// precedent in the crate wins.
    #[must_use]
    pub fn identity() -> Mat4 {
        Mat4 {
            cols: [
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0, //
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    /// The sixteen floats in column-major order, for the GL upload.
    ///
    /// Coerces to `&[f32]` at the call site with no copy and no cast. This is
    /// the only accessor: a matrix that can also be read any other way is a
    /// matrix with two conventions to disagree.
    #[must_use]
    pub fn as_slice(&self) -> &[f32; 16] {
        &self.cols
    }

    /// The matrix product `self * rhs`.
    ///
    /// Post-multiplying: `projection.multiply(&view).multiply(&model)` is
    /// `P * V * M`, and a point it transforms is modelled, then viewed, then
    /// projected. The two wrong orders, `M * V * P` and `V * P * M`, both
    /// compile and both draw a plausible-looking wrong scene, which is why the
    /// order has a test that fails when it is reversed rather than a comment
    /// asking it not to be.
    ///
    /// Written out directly rather than composed from the convenience ops
    /// below, so that test is a cross-check between two implementations and
    /// not a tautology.
    #[must_use]
    pub fn multiply(&self, rhs: &Mat4) -> Mat4 {
        let mut cols = [0.0; 16];
        let mut c = 0;
        while c < 4 {
            let mut r = 0;
            while r < 4 {
                cols[c * 4 + r] = self.cols[r] * rhs.cols[c * 4]
                    + self.cols[4 + r] * rhs.cols[c * 4 + 1]
                    + self.cols[8 + r] * rhs.cols[c * 4 + 2]
                    + self.cols[12 + r] * rhs.cols[c * 4 + 3];
                r += 1;
            }
            c += 1;
        }
        Mat4 { cols }
    }

    /// `self` translated by `offset`: `self * T`.
    ///
    /// Post-multiplying, so `identity().translated(t).rotated_z(r).scaled(s)`
    /// is `T * R * S` and a point is scaled, then rotated, then translated —
    /// the order a caller means. The last column becomes `self` applied to
    /// `[ox, oy, oz, 1]`; columns 0-2 are untouched, which is what catches a
    /// row-major mistake in the test.
    #[must_use]
    pub fn translated(&self, offset: [f32; 3]) -> Mat4 {
        let mut cols = self.cols;
        let mut r = 0;
        while r < 4 {
            cols[12 + r] = self.cols[r] * offset[0]
                + self.cols[4 + r] * offset[1]
                + self.cols[8 + r] * offset[2]
                + self.cols[12 + r];
            r += 1;
        }
        Mat4 { cols }
    }

    /// `self` scaled per axis: `self * S`.
    ///
    /// Columns 0-2 are scaled by `factor[0..2]` and the translation column is
    /// untouched. A zero factor collapses its axis and makes the matrix
    /// non-invertible — that is representable here on purpose, and the future
    /// normal-matrix requirement on task 37 must know it can happen.
    #[must_use]
    pub fn scaled(&self, factor: [f32; 3]) -> Mat4 {
        let mut cols = self.cols;
        let mut i = 0;
        while i < 4 {
            cols[i] *= factor[0];
            cols[4 + i] *= factor[1];
            cols[8 + i] *= factor[2];
            i += 1;
        }
        Mat4 { cols }
    }

    /// `self` rotated about the x axis: `self * Rx`.
    ///
    /// Required in addition to the z rotation: the car's wheels spin about x,
    /// and a `Mat4` with only a z rotation would leave task 40
    /// hand-multiplying an axis-angle matrix with no test. All three rotations
    /// are the same one-line standard rotation; three methods for three axes
    /// is not a seam.
    #[must_use]
    pub fn rotated_x(&self, radians: f32) -> Mat4 {
        let (s, c) = radians.sin_cos();
        let mut cols = self.cols;
        let mut r = 0;
        while r < 4 {
            let y = self.cols[4 + r];
            let z = self.cols[8 + r];
            cols[4 + r] = c * y + s * z;
            cols[8 + r] = c * z - s * y;
            r += 1;
        }
        Mat4 { cols }
    }

    /// `self` rotated about the y axis: `self * Ry`.
    ///
    /// The rotation the car's placement needs: yaw is about y. See
    /// [`Mat4::rotated_x`] for why all three axes exist.
    #[must_use]
    pub fn rotated_y(&self, radians: f32) -> Mat4 {
        let (s, c) = radians.sin_cos();
        let mut cols = self.cols;
        let mut r = 0;
        while r < 4 {
            let x = self.cols[r];
            let z = self.cols[8 + r];
            cols[r] = c * x - s * z;
            cols[8 + r] = s * x + c * z;
            r += 1;
        }
        Mat4 { cols }
    }

    /// `self` rotated about the z axis: `self * Rz`.
    ///
    /// The rotation that makes `Transform` expressible at all — see
    /// `Transform::to_matrix` for the axis, the handedness, and the y-flip
    /// that is deliberately not baked in.
    #[must_use]
    pub fn rotated_z(&self, radians: f32) -> Mat4 {
        let (s, c) = radians.sin_cos();
        let mut cols = self.cols;
        let mut r = 0;
        while r < 4 {
            let x = self.cols[r];
            let y = self.cols[4 + r];
            cols[r] = c * x + s * y;
            cols[4 + r] = c * y - s * x;
            r += 1;
        }
        Mat4 { cols }
    }

    /// Transforms a homogeneous vector: no division, no `Option`, total.
    ///
    /// This exists beside `transform_point` because chaining transforms must
    /// not divide by `w` at each step: only the final projection's `w` is a
    /// perspective divide, and dividing mid-chain changes the point the later
    /// matrices see.
    #[must_use]
    pub fn transform_vec4(&self, v: [f32; 4]) -> [f32; 4] {
        let mut out = [0.0; 4];
        let mut r = 0;
        while r < 4 {
            out[r] = self.cols[r] * v[0]
                + self.cols[4 + r] * v[1]
                + self.cols[8 + r] * v[2]
                + self.cols[12 + r] * v[3];
            r += 1;
        }
        out
    }

    /// Transforms a point, dividing out `w`.
    ///
    /// `None` only when `w` is zero or non-finite — a point at the camera
    /// plane, where dividing would be a division by zero. A point **behind**
    /// the camera has `w < 0` and returns `Some` with a finite result: that is
    /// clipping, not an error, and GL discards the primitive from the clip
    /// coordinates rather than from a `None` a caller would act on.
    #[must_use]
    pub fn transform_point(&self, point: [f32; 3]) -> Option<[f32; 3]> {
        let clip = self.transform_vec4([point[0], point[1], point[2], 1.0]);
        let w = clip[3];
        if w == 0.0 || !w.is_finite() {
            return None;
        }
        Some([clip[0] / w, clip[1] / w, clip[2] / w])
    }

    /// An OpenGL perspective projection, or `None` on degenerate input.
    ///
    /// Produces clip space with `z` in `[-1, 1]`, because GLES 3.1's clip
    /// volume is `[-1, 1]` and it is that which makes window depth `0.5` for
    /// `z_ndc = 0.0`. The `[0, 1]` D3D/Vulkan form is named here as the
    /// mistake it is: under it the near/far test's `-1.0` and `1.0` both fail.
    ///
    /// `None` unless `0.0 < fov_y_radians < PI`, `aspect > 0.0`,
    /// `0.0 < near < far`, and every argument finite. A zero field of view
    /// would otherwise divide by `tan(0.0) = 0.0`, which in Rust is `inf`
    /// rather than a panic — the matrix uploads, every triangle vanishes, and
    /// there is no GL error.
    #[must_use]
    pub fn perspective(fov_y_radians: f32, aspect: f32, near: f32, far: f32) -> Option<Mat4> {
        if !fov_y_radians.is_finite()
            || !aspect.is_finite()
            || !near.is_finite()
            || !far.is_finite()
        {
            return None;
        }
        if fov_y_radians <= 0.0 || fov_y_radians >= PI {
            return None;
        }
        if aspect <= 0.0 {
            return None;
        }
        if near <= 0.0 || near >= far {
            return None;
        }
        let f = 1.0 / (fov_y_radians * 0.5).tan();
        let depth = (far + near) / (near - far);
        let shift = (2.0 * far * near) / (near - far);
        Some(Mat4 {
            cols: [
                f / aspect,
                0.0,
                0.0,
                0.0, //
                0.0,
                f,
                0.0,
                0.0, //
                0.0,
                0.0,
                depth,
                -1.0, //
                0.0,
                0.0,
                shift,
                0.0,
            ],
        })
    }

    /// An orthographic projection, or `None` on a degenerate box.
    ///
    /// Takes **signed** `near` and `far` as view-space distances along `-z`
    /// (OpenGL convention: the camera looks along `-z`, so visible distances
    /// are negative `z`). **The 2D box is `near = -1.0, far = 1.0`**, and it
    /// must be: passing the positive, obvious-looking `0.1, 100.0` maps `z = 0`
    /// to `z' = -1.002` — the near plane — instead of `0.0`, which is the
    /// value task 34's depth arithmetic is written against. A test asserts the
    /// trap rather than leaving it as a warning nobody reads.
    ///
    /// `None` unless `right != left`, `top != bottom`, `far != near`, and every
    /// argument finite. A reversed box (`right < left`) is representable — it
    /// mirrors — so only the empty box is refused.
    #[must_use]
    pub fn orthographic(
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    ) -> Option<Mat4> {
        if !left.is_finite()
            || !right.is_finite()
            || !bottom.is_finite()
            || !top.is_finite()
            || !near.is_finite()
            || !far.is_finite()
        {
            return None;
        }
        if right == left || top == bottom || far == near {
            return None;
        }
        let sx = 2.0 / (right - left);
        let sy = 2.0 / (top - bottom);
        let sz = -2.0 / (far - near);
        let tx = -(right + left) / (right - left);
        let ty = -(top + bottom) / (top - bottom);
        let tz = -(far + near) / (far - near);
        Some(Mat4 {
            cols: [
                sx, 0.0, 0.0, 0.0, //
                0.0, sy, 0.0, 0.0, //
                0.0, 0.0, sz, 0.0, //
                tx, ty, tz, 1.0,
            ],
        })
    }
}

impl Transform {
    /// Converts this 2D transform into the matrix the GPU path consumes:
    /// `T * Rz * S` in the plane the transform already describes.
    ///
    /// `Transform` itself is **unchanged** by this task — five fields, its
    /// `identity()`, its `Default`, its field-by-field `Interpolate` — and
    /// `property.rs` is not edited, so the crate's lowest layer gains no
    /// dependency on `render`. This conversion is the first edge out of
    /// `Transform` in the crate's history, and no widget's paint path calls it:
    /// a test of it proves the conversion and nothing else.
    ///
    /// The rotation axis is **z**, which is documented nowhere in the crate and
    /// is derivable anyway: a 2D rotation has no other axis to be about. The
    /// crate's window space is **y-down** (`Rect.y` is an offset from the top
    /// edge), so a positive rotation turns **clockwise on screen without a
    /// flip and counter-clockwise with the four 2D shaders' `-clip.y`**.
    /// Neither direction is behaviour the crate has today, because no 2D
    /// transform is drawn anywhere and the choice is task 37's and 40's — so
    /// the y-flip is deliberately **not** baked into this matrix. All four
    /// existing shaders apply it themselves, and a matrix carrying it would be
    /// wrong for every one of them.
    #[must_use]
    pub fn to_matrix(&self) -> Mat4 {
        let (s, c) = self.rotation.sin_cos();
        Mat4 {
            cols: [
                c * self.sx,
                s * self.sx,
                0.0,
                0.0, //
                -s * self.sy,
                c * self.sy,
                0.0,
                0.0, //
                0.0,
                0.0,
                1.0,
                0.0, //
                self.tx,
                self.ty,
                0.0,
                1.0,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::FRAC_PI_2;

    /// `f32::cos(PI / 2)` is `6.12e-17`, not `0.0`: an exact `PartialEq` on
    /// `[f32; 16]` is a suite that fails on the last bit of a `sin_cos`.
    /// Every matrix test below goes through `close` at `1e-5`, and every
    /// expectation is a hand-written literal — never another `Mat4` method's
    /// output, which would compare two implementations of the same arithmetic
    /// and catch nothing.
    ///
    /// The comparisons count matches rather than asserting booleans: `assert!`
    /// is unavailable in this file by the task's token rule, and clippy
    /// refuses `assert_eq!` with a boolean literal, so a mismatch fails on the
    /// count with both arrays in the message.
    fn close(got: f32, want: f32) -> bool {
        (got - want).abs() <= 0.00001
    }

    /// Asserts all sixteen floats against a hand-written column-major literal.
    fn assert_matrix_eq(got: &Mat4, want: [f32; 16]) {
        let matching = got
            .cols
            .iter()
            .zip(want.iter())
            .filter(|(g, w)| close(**g, **w))
            .count();
        assert_eq!(matching, 16, "got {:?}, want {:?}", got.cols, want);
    }

    /// Asserts a transformed homogeneous vector against a hand-written literal.
    fn assert_vec4_eq(got: [f32; 4], want: [f32; 4]) {
        let matching = got
            .iter()
            .zip(want.iter())
            .filter(|(g, w)| close(**g, **w))
            .count();
        assert_eq!(matching, 4, "got {got:?}, want {want:?}");
    }

    /// Asserts a transformed point against a hand-written literal.
    fn assert_point_eq(got: [f32; 3], want: [f32; 3]) {
        let matching = got
            .iter()
            .zip(want.iter())
            .filter(|(g, w)| close(**g, **w))
            .count();
        assert_eq!(matching, 3, "got {got:?}, want {want:?}");
    }

    #[test]
    fn identity_leaves_a_point_where_it_was() {
        let point = Mat4::identity().transform_point([1.0, 2.0, 3.0]);
        let Some(mapped) = point else {
            assert_eq!("identity", "returned None for a finite point");
            return;
        };
        assert_point_eq(mapped, [1.0, 2.0, 3.0]);
        assert_matrix_eq(
            &Mat4::identity(),
            [
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0, //
                0.0, 0.0, 0.0, 1.0,
            ],
        );
    }

    #[test]
    fn translate_puts_the_translation_in_the_last_column() {
        // Columns 0-2 are the identity — which is what GLSL's `m[c][r]`
        // indexing means — and the translation is the whole of column 3.
        // A row-major mistake writes it into `cols[3]`, `cols[7]`, `cols[11]`
        // instead, and both this test and `the_sixteen_floats_are_column_major`
        // fail on that mutation.
        let moved = Mat4::identity().translated([4.0, 5.0, 6.0]);
        assert_matrix_eq(
            &moved,
            [
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0, //
                4.0, 5.0, 6.0, 1.0,
            ],
        );
    }

    #[test]
    fn a_quarter_turn_about_z_takes_x_onto_y() {
        let turned = Mat4::identity().rotated_z(FRAC_PI_2);
        let point = turned.transform_point([1.0, 0.0, 0.0]);
        let Some(mapped) = point else {
            assert_eq!("rotated_z", "returned None for a finite point");
            return;
        };
        assert_point_eq(mapped, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn a_quarter_turn_about_y_takes_z_onto_x() {
        // Yaw: the rotation the car's placement needs.
        let turned = Mat4::identity().rotated_y(FRAC_PI_2);
        let point = turned.transform_point([0.0, 0.0, 1.0]);
        let Some(mapped) = point else {
            assert_eq!("rotated_y", "returned None for a finite point");
            return;
        };
        assert_point_eq(mapped, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn a_quarter_turn_about_x_takes_y_onto_z() {
        // Wheel spin: the rotation the wheels need.
        let turned = Mat4::identity().rotated_x(FRAC_PI_2);
        let point = turned.transform_point([0.0, 1.0, 0.0]);
        let Some(mapped) = point else {
            assert_eq!("rotated_x", "returned None for a finite point");
            return;
        };
        assert_point_eq(mapped, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn scale_multiplies_each_axis() {
        let scaled = Mat4::identity().scaled([2.0, 3.0, 4.0]);
        let point = scaled.transform_point([1.0, 1.0, 1.0]);
        let Some(mapped) = point else {
            assert_eq!("scaled", "returned None for a finite point");
            return;
        };
        assert_point_eq(mapped, [2.0, 3.0, 4.0]);
    }

    #[test]
    fn a_zero_scale_collapses_the_axis_it_was_given() {
        // A zero scale makes the matrix non-invertible, and that is
        // representable here on purpose: the future normal-matrix requirement
        // on task 37 must know a collapsed axis can arrive.
        let scaled = Mat4::identity().scaled([0.0, 2.0, 3.0]);
        let point = scaled.transform_point([5.0, 1.0, 1.0]);
        let Some(mapped) = point else {
            assert_eq!("zero-scaled", "returned None for a finite point");
            return;
        };
        assert_point_eq(mapped, [0.0, 2.0, 3.0]);
    }

    #[test]
    fn multiply_is_the_matrix_product_and_its_order_is_the_glu_one() {
        // For a perspective, a rotation and a translation,
        // `mvp.transform_vec4(point)` equals the three applied in turn —
        // modelled, then viewed, then projected. `transform_vec4` is used
        // rather than `transform_point` because only the final projection's
        // `w` is a perspective divide: dividing mid-chain changes the point
        // the later matrices see. Reversing the product to
        // `model.multiply(&view).multiply(&projection)` fails this test.
        let Some(projection) = Mat4::perspective(FRAC_PI_2, 1.0, 1.0, 2.0) else {
            assert_eq!("perspective", "returned None for valid arguments");
            return;
        };
        let view = Mat4::identity().rotated_z(FRAC_PI_2);
        let model = Mat4::identity().translated([1.0, 0.0, 0.0]);
        let mvp = projection.multiply(&view).multiply(&model);
        let point = [1.0, 0.0, -1.0, 1.0];
        let got = mvp.transform_vec4(point);
        // Hand-computed: the model sends x 1.0 to 2.0; the quarter turn about
        // z sends [2.0, 0.0] to [0.0, 2.0]; the projection maps the near plane
        // (z = -1.0) to z = -1.0 with w = 1.0.
        assert_vec4_eq(got, [0.0, 2.0, -1.0, 1.0]);
        let chained = projection.transform_vec4(view.transform_vec4(model.transform_vec4(point)));
        assert_vec4_eq(got, chained);
    }

    #[test]
    fn the_transforms_compose_translate_rotate_scale_in_that_order() {
        // `translated(t).rotated_z(r).scaled(s)` is `T * R * S`: a point is
        // scaled, then rotated, then translated. Hand-computed: [1, 0, 0]
        // scales to [2, 0, 0], turns a quarter about z to [0, 2, 0], and moves
        // by [10, 3, 0] to [10, 5, 0].
        let composed = Mat4::identity()
            .translated([10.0, 3.0, 0.0])
            .rotated_z(FRAC_PI_2)
            .scaled([2.0, 1.0, 1.0]);
        let point = composed.transform_point([1.0, 0.0, 0.0]);
        let Some(mapped) = point else {
            assert_eq!("composed", "returned None for a finite point");
            return;
        };
        assert_point_eq(mapped, [10.0, 5.0, 0.0]);
    }

    #[test]
    fn transform_to_matrix_agrees_with_hand_computed_window_space_values() {
        // tx = 10.0, sx = sy = 2.0, no rotation: [1, 1, 0] scales to
        // [2, 2, 0] and moves to [12, 2, 0].
        let transform = Transform {
            tx: 10.0,
            ty: 0.0,
            sx: 2.0,
            sy: 2.0,
            rotation: 0.0,
        };
        let point = transform.to_matrix().transform_point([1.0, 1.0, 0.0]);
        let Some(mapped) = point else {
            assert_eq!("to_matrix", "returned None for a finite point");
            return;
        };
        assert_point_eq(mapped, [12.0, 2.0, 0.0]);
    }

    #[test]
    fn perspective_puts_the_near_plane_at_minus_one_and_the_far_plane_at_one() {
        // With near = 1.0, far = 2.0 the two planes map to z = -1.0 and
        // z = 1.0. These are the two values that fail if the `[0, 1]`
        // D3D/Vulkan convention is used by mistake.
        let Some(projection) = Mat4::perspective(FRAC_PI_2, 1.0, 1.0, 2.0) else {
            assert_eq!("perspective", "returned None for valid arguments");
            return;
        };
        let near = projection.transform_point([0.0, 0.0, -1.0]);
        let Some(near_mapped) = near else {
            assert_eq!("near plane", "mapped to w = 0");
            return;
        };
        assert_point_eq(near_mapped, [0.0, 0.0, -1.0]);
        let far = projection.transform_point([0.0, 0.0, -2.0]);
        let Some(far_mapped) = far else {
            assert_eq!("far plane", "mapped to w = 0");
            return;
        };
        assert_point_eq(far_mapped, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn perspective_is_none_for_every_degenerate_argument() {
        use core::f32::consts::PI;
        // Each row violates exactly one precondition; each is `None`.
        let degenerate = [
            Mat4::perspective(0.0, 1.0, 0.1, 20.0),
            Mat4::perspective(PI, 1.0, 0.1, 20.0),
            Mat4::perspective(-1.0, 1.0, 0.1, 20.0),
            Mat4::perspective(FRAC_PI_2, 0.0, 0.1, 20.0),
            Mat4::perspective(FRAC_PI_2, 1.0, 0.0, 20.0),
            Mat4::perspective(FRAC_PI_2, 1.0, 20.0, 20.0),
            Mat4::perspective(FRAC_PI_2, 1.0, 30.0, 20.0),
            Mat4::perspective(f32::NAN, 1.0, 0.1, 20.0),
            Mat4::perspective(FRAC_PI_2, f32::INFINITY, 0.1, 20.0),
        ];
        let rejected = degenerate.iter().filter(|r| r.is_none()).count();
        assert_eq!(
            rejected,
            degenerate.len(),
            "a degenerate perspective was accepted"
        );
        let Some(valid) = Mat4::perspective(FRAC_PI_2, 1.0, 0.1, 20.0) else {
            assert_eq!("perspective", "returned None for valid arguments");
            return;
        };
        assert_eq!(valid.as_slice().len(), 16);
    }

    #[test]
    fn orthographic_reproduces_the_window_to_clip_mapping_the_four_shaders_write() {
        // With left = 0, right = 640, bottom = 0, top = 480, near = -1,
        // far = 1, the corners and the centre land where the four 2D vertex
        // shaders put them — including `z == 0.0`, the value all four
        // hard-code. This is the test that ties the maths to the shipped
        // pipeline. The off-plane point pins the sign convention: the box maps
        // `z = 1.0` to `-1.0` (the projection negates `z`), while swapping near
        // and far to `(1.0, -1.0)` keeps every `z = 0` mapping (the box is
        // symmetric about the plane) and flips this one to `+1.0`, so the
        // mutation fails here rather than on a `z == 0.0` it cannot move.
        let Some(projection) = Mat4::orthographic(0.0, 640.0, 0.0, 480.0, -1.0, 1.0) else {
            assert_eq!("orthographic", "returned None for a valid box");
            return;
        };
        let corners = [
            ([0.0, 0.0, 0.0], [-1.0, -1.0, 0.0]),
            ([320.0, 240.0, 0.0], [0.0, 0.0, 0.0]),
            ([640.0, 480.0, 0.0], [1.0, 1.0, 0.0]),
            ([0.0, 0.0, 1.0], [-1.0, -1.0, -1.0]),
        ];
        for (input, want) in corners.iter() {
            let mapped = projection.transform_point(*input);
            let Some(got) = mapped else {
                assert_eq!("orthographic corner", "returned None mid-box");
                return;
            };
            assert_point_eq(got, *want);
        }
    }

    #[test]
    fn orthographic_with_positive_distances_puts_z_zero_on_the_near_plane() {
        // The sign convention as a fact, not a warning: `near = 0.1`,
        // `far = 100.0` maps `z = 0` to `-(100.1 / 99.9) = -1.002` — past the
        // near plane, so the fragment is clipped — instead of `0.0`. The
        // hand-written `-1.002` is within `close`'s `1e-5` of the exact
        // `-1.0020020`, and being past `-1.0` is what makes it a clip.
        let Some(projection) = Mat4::orthographic(0.0, 640.0, 0.0, 480.0, 0.1, 100.0) else {
            assert_eq!("orthographic", "returned None for a valid box");
            return;
        };
        let mapped = projection.transform_point([320.0, 240.0, 0.0]);
        let Some(got) = mapped else {
            assert_eq!("positive-box", "returned None mid-box");
            return;
        };
        assert_point_eq(got, [0.0, 0.0, -1.002]);
    }

    #[test]
    fn orthographic_is_none_for_an_empty_box() {
        let degenerate = [
            Mat4::orthographic(1.0, 1.0, 0.0, 480.0, -1.0, 1.0),
            Mat4::orthographic(0.0, 640.0, 2.0, 2.0, -1.0, 1.0),
            Mat4::orthographic(0.0, 640.0, 0.0, 480.0, 1.0, 1.0),
        ];
        let rejected = degenerate.iter().filter(|r| r.is_none()).count();
        assert_eq!(
            rejected,
            degenerate.len(),
            "a degenerate orthographic box was accepted"
        );
        let Some(valid) = Mat4::orthographic(0.0, 640.0, 0.0, 480.0, -1.0, 1.0) else {
            assert_eq!("orthographic", "returned None for a valid box");
            return;
        };
        assert_eq!(valid.as_slice().len(), 16);
    }

    #[test]
    fn transform_point_is_none_at_the_camera_plane_and_keeps_points_behind_it() {
        // A point at the camera plane has `w = 0`: dividing would be a
        // division by zero, so this is `None`. A point behind the camera has
        // `w < 0` and returns `Some` with a finite result — that is clipping,
        // not an error, and GL discards it from the clip coordinates.
        let Some(projection) = Mat4::perspective(FRAC_PI_2, 1.0, 1.0, 2.0) else {
            assert_eq!("perspective", "returned None for valid arguments");
            return;
        };
        assert_eq!(
            projection.transform_point([0.0, 0.0, 0.0]),
            None,
            "the camera plane must map to None"
        );
        let behind = projection.transform_point([0.0, 0.0, 1.0]);
        let Some(mapped) = behind else {
            assert_eq!("behind the camera", "returned None; clipping is Some");
            return;
        };
        let finite = mapped.iter().filter(|v| v.is_finite()).count();
        assert_eq!(
            finite, 3,
            "a point behind the camera maps to a finite point, got {mapped:?}"
        );
    }

    #[test]
    fn the_sixteen_floats_are_column_major() {
        // The whole array for `translated([1.0, 2.0, 3.0])` as one hand-written
        // column-major literal: the translation in `cols[12..15]`, identity
        // everywhere else. Transposing the write into `cols[3]`, `cols[7]`,
        // `cols[11]` fails this test and
        // `translate_puts_the_translation_in_the_last_column` together.
        assert_eq!(size_of::<Mat4>(), 64);
        let moved = Mat4::identity().translated([1.0, 2.0, 3.0]);
        assert_eq!(moved.as_slice().len(), 16);
        assert_matrix_eq(
            &moved,
            [
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0, //
                1.0, 2.0, 3.0, 1.0,
            ],
        );
    }
}
