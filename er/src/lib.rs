#![no_std]
#![doc = include_str!("../docs/examples.md")]
#![doc = include_str!("../docs/macros.md")]

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
pub use lines::ErLineError;
#[doc(inline)]
pub use snapshot::*;
#[doc(inline)]
pub use traits::*;
#[doc(inline)]
pub use types::*;
#[cfg(feature = "test")]
#[doc(inline)]
pub use types_test::*;
#[doc(inline)]
pub use walk::{ErEntries, ErEntry, ErEntryKind, ErFindAll, ErNodes, ErSources};
