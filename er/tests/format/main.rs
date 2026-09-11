#![cfg(feature = "macros")]

pub mod lines;
pub mod presentation;
pub mod rendering;

// A real caller with src in its path, to exercise path shortening.
#[cfg(feature = "src_locations")]
#[path = "src/paths.rs"]
pub mod paths;
