#![no_std]
#![cfg_attr(er_unsync, doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")))]
#![cfg_attr(not(er_unsync), doc = include_str!("../docs/examples.md"))]
#![cfg_attr(not(er_unsync), doc = include_str!("../docs/macros.md"))]
#![cfg_attr(not(er_unsync), doc = include_str!("../docs/good-and-bad.md"))]

extern crate alloc;
#[cfg(feature = "stack_traces")]
extern crate std;

// Only the items re-exported below are the API, the modules are just where they live.
#[doc(hidden)]
pub mod aggregate;
#[doc(hidden)]
pub mod impls;
#[doc(hidden)]
pub mod internal;
pub(crate) use internal::*;
#[doc(hidden)]
pub mod lines;
#[cfg(feature = "macros")]
#[doc(hidden)]
pub mod macros;
#[doc(hidden)]
pub mod render;
#[doc(hidden)]
pub mod snapshot;
#[doc(hidden)]
pub mod traits;
#[doc(hidden)]
pub mod types;
#[doc(hidden)]
pub mod types_test;
#[doc(hidden)]
pub mod walk;
#[doc(hidden)]
mod wrapped;

#[cfg(feature = "macros")]
#[doc(hidden)]
pub use er_macros::{__er_all_results, __er_assert_err, __er_bail_nodes, __er_try_results};
#[cfg(feature = "macros")]
#[doc(inline)]
pub use er_macros::{Er, ErFormat};
#[doc(inline)]
pub use lines::ErLineError;
#[doc(inline)]
pub use snapshot::*;
#[doc(inline)]
pub use traits::*;
#[doc(inline)]
pub use types::*;
#[doc(inline)]
pub use types_test::*;
#[doc(inline)]
pub use walk::{ErEntries, ErEntry, ErEntryKind, ErFindAll, ErNodes, ErSources, MAX_SOURCE_HOPS};
#[doc(inline)]
pub use wrapped::{ErWrapExt, ErWrapIteratorExt, ErWrapResultExt};
