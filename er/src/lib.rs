#![no_std]
#![cfg_attr(er_unsync, doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")))]
#![cfg_attr(not(er_unsync), doc = include_str!("../docs/examples.md"))]
#![cfg_attr(not(er_unsync), doc = include_str!("../docs/macros.md"))]

extern crate alloc;
#[cfg(feature = "stack_traces")]
extern crate std;

pub mod aggregate;
pub mod impls;
#[cfg(feature = "lazy")]
pub mod lazy;
pub mod lines;
#[cfg(feature = "macros")]
pub mod macros;
pub mod render;
pub mod snapshot;
pub mod traits;
pub mod types;
#[cfg(feature = "test")]
pub mod types_test;
pub mod walk;

#[cfg(all(feature = "macros", not(er_unsync)))]
#[doc(inline)]
pub use er_macros::Er;
#[cfg(feature = "macros")]
#[doc(inline)]
pub use er_macros::ErFormat;
#[cfg(all(feature = "macros", er_unsync))]
#[doc(inline)]
pub use er_macros::ErUnsync as Er;
#[cfg(feature = "macros")]
#[doc(hidden)]
pub use er_macros::{__er_all_results, __er_try_results};
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
