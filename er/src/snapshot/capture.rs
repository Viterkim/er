#[cfg(feature = "src_locations")]
use crate::ErSnapshotLocation;
use crate::{ErSnapshot, ErSnapshotEntry, ErTree};
#[cfg(feature = "src_locations")]
use alloc::string::ToString;
use alloc::{string::String, vec::Vec};
use core::{error::Error, fmt};

impl<E: Error + 'static> ErTree<E> {
    /// Save the messages, tree structure and locations, without keeping the errors.
    /// A broken formatter's partial message is replaced with `ER_FMT_FAILED`.
    pub fn er_snapshot(&self) -> ErSnapshot {
        let mut entries = Vec::new();

        for entry in self.er_entries() {
            let mut message = String::new();
            let result = fmt::write(&mut message, format_args!("{}", entry.error));
            if result.is_err() {
                message.clear();
                message.push_str("ER_FMT_FAILED");
            }

            #[cfg(feature = "src_locations")]
            let src_location = entry.src_location.map(|location| {
                let file = location.file().to_string();
                let line = location.line();
                let column = location.column();

                ErSnapshotLocation { file, line, column }
            });

            entries.push(ErSnapshotEntry {
                message,
                kind: entry.kind,
                index: entry.index,
                parent: entry.parent,
                depth: entry.depth,
                is_last: entry.is_last,
                source_truncated: entry.source_truncated,
                #[cfg(feature = "src_locations")]
                src_location,
            });
        }

        ErSnapshot { entries }
    }
}
