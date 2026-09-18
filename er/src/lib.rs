#![no_std]
#![cfg_attr(feature = "macros", doc = include_str!("../docs/examples.md"))]
#![cfg_attr(
    not(feature = "macros"),
    doc = "Error trees with context. Enable `macros` for the derives and their examples. The runtime works with your own error impls too, start with [`ErError`] and [`ErContext`]."
)]

extern crate alloc;

pub mod aggregate;
pub mod impls;
#[cfg(feature = "lazy")]
pub mod lazy;
pub mod lines;
pub mod macros;
pub mod render;
pub mod snapshot;
pub mod traits;
pub mod types;
mod types_helpers;
#[cfg(feature = "test")]
pub mod types_test;
pub mod walk;

#[cfg(feature = "macros")]
#[doc(inline)]
pub use er_macros::{Er, ErFormat};
#[cfg(feature = "lazy")]
#[doc(inline)]
pub use lazy::{ErLazy, ErLazyError};
#[doc(inline)]
pub use traits::*;
#[doc(inline)]
pub use types::*;
#[cfg(feature = "test")]
#[doc(inline)]
pub use types_test::*;
#[doc(inline)]
pub use walk::MAX_SOURCE_HOPS;
