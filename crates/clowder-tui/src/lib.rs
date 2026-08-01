//! clowder's terminal shell, as a library.
//!
//! The binary is a thin shim over `runtime::run`. The library target exists so
//! integration tests can address the real module tree — without it each test
//! must reconstruct a crate root by compiling source files in by path, which
//! breaks the moment any module refers to a sibling through `crate::`.

pub mod nav;
pub mod render;
pub mod runtime;
pub mod theme;
