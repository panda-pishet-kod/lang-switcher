//! The interface shell of release 1.8.0 (решения 208.163, 208.169): one style table, one look per
//! element, a window described as data, drawn by the system's Direct2D and DirectWrite.
//!
//! The file boundaries below are hard: each file names what it knows and what it must not know,
//! and `tests\ui_guard.rs` holds them.
//!
//! ⚠ Inside this module `windows` is the submodule of window descriptions; the crate of the same
//! name is reached as `::windows`.

pub mod art;
pub mod bench;
pub mod canvas;
pub mod elements;
pub mod fit;
pub mod host;
pub mod layout;
pub mod scene;
pub mod text;
pub mod tokens;
pub mod windows;
