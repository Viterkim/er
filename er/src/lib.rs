#![no_std]
#![cfg_attr(feature = "macros", doc = include_str!("../docs/examples.md"))]
#![cfg_attr(
    not(feature = "macros"),
    doc = "Error trees with context. Enable `macros` for the derives and their examples. The runtime works with your own error impls too, start with [`ErError`] and [`ErResult`]."
)]

extern crate alloc;

pub mod aggregate;
pub mod impls;
pub mod lines;
pub mod macros;
pub mod render;
pub mod snapshot;
pub mod traits;
pub mod types;
#[cfg(feature = "test")]
pub mod types_test;
pub mod walk;

#[cfg(feature = "macros")]
#[doc(inline)]
pub use er_macros::{Er, ErFormat};
#[doc(inline)]
pub use lines::LineError;
#[doc(inline)]
pub use traits::*;
#[doc(inline)]
pub use types::*;
#[cfg(feature = "test")]
#[doc(inline)]
pub use types_test::*;
#[doc(inline)]
pub use walk::{ErEntries, ErNodes, ErSources, MAX_SOURCE_HOPS};
