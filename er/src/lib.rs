#![no_std]
#![cfg_attr(er_unsync, doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")))]
#![cfg_attr(not(er_unsync), doc = include_str!("../docs/examples.md"))]
#![cfg_attr(not(er_unsync), doc = include_str!("../docs/good-and-bad.md"))]
#![cfg_attr(not(er_unsync), doc = include_str!("../docs/macros.md"))]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

// Only the items re-exported below are the API, the modules are just where they live.
mod aggregate;
mod impls;
mod internal;
pub(crate) use internal::*;
mod lines;
#[cfg(feature = "macros")]
mod macros;
mod render;
mod snapshot;
mod traits;
mod types;
mod types_main;
mod types_test;
mod walk;
mod wrapped;

/// Internal modules used by `er` macros and advanced tooling.
#[doc(hidden)]
pub mod er_internal {
    pub use crate::internal::*;

    pub mod aggregate {
        pub use crate::aggregate::*;
    }
    pub mod impls {
        #[cfg(feature = "macros")]
        pub mod format {
            pub use crate::impls::format::*;
        }
        pub mod saved {
            pub use crate::impls::saved::*;
        }
        #[cfg(feature = "macros")]
        pub mod source {
            pub use crate::impls::source::*;
        }
        #[cfg(feature = "macros")]
        pub mod testing {
            pub use crate::impls::testing::*;
        }
    }
    pub mod lines {
        pub use crate::lines::*;
    }
    pub mod render {
        pub use crate::render::*;
        pub mod report {
            pub use crate::render::report::*;
        }
    }
    pub mod walk {
        pub use crate::walk::*;
    }
}

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
pub use types_main::*;
#[doc(inline)]
pub use types_test::*;
#[doc(inline)]
pub use walk::{ErEntries, ErEntry, ErEntryKind, ErFindAll, ErNodes, ErSources};
#[doc(inline)]
pub use wrapped::{ErWrapExt, ErWrapIteratorExt, ErWrapResultExt};
