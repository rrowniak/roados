//! UI primitives for the roados infotainment interface.
//!
//! A retained-mode widget tree rendered over OpenGL ES 3.1. The module list
//! below is `doc/ui/PRIMITIVES_ARCHITECTURE.md` § *Module Layout*; the
//! entry-point filename and what that document says about it are recorded in
//! `doc/ui/IMPLEMENTATION_STATE.md` § *Deviations from the spec, and why*.
//!
//! The stages of the frame pipeline each module owns — input, animation,
//! layout, paint, batching, submission — are listed in that document's *Frame
//! lifecycle*. Some are still stubs; `layout`, `node` and `paint` are not, and
//! a module's own docs say which it is.
#![warn(missing_docs)]

pub mod animation;
pub mod arena;
pub mod batch;
pub mod font;
pub mod input;
pub mod layout;
pub mod node;
pub mod paint;
pub mod property;
pub mod render;
pub mod theme;
pub mod widgets;
