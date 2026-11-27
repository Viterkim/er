use crate::InternalReport;
use crate::render::{REPORT_CAPACITY, write_report};
use alloc::string::String;
use core::fmt;

struct ReportBuffer<'a> {
    text: &'a mut String,
    location_bytes: usize,
}
impl fmt::Write for ReportBuffer<'_> {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        // Leave room for the location so a long message doesn't double in size for one more line number.
        if value.len() > self.text.capacity() - self.text.len() {
            self.text
                .reserve(value.len().saturating_add(self.location_bytes));
        }
        self.text.push_str(value);
        Ok(())
    }
}

impl InternalReport<'_> {
    pub(crate) fn write_to(&self, writer: &mut dyn fmt::Write) -> fmt::Result {
        write_report(
            writer,
            self.top,
            self.nodes,
            self.saved,
            self.extra,
            self.src_location,
        )
    }

    #[inline(never)]
    pub(crate) fn render_string(&self) -> String {
        let mut text;
        let result = if self.nodes.is_empty() && self.saved.is_empty() && self.extra.is_empty() {
            text = String::new();
            self.write_to(&mut ReportBuffer {
                text: &mut text,
                location_bytes: self.src_location.map_or(16, |loc| loc.file().len() + 32),
            })
        } else {
            // Every direct child prints at least a newline and a branch.
            let branch_bytes = 4 * self.nodes.len();
            let capacity = if branch_bytes <= REPORT_CAPACITY {
                REPORT_CAPACITY
            } else {
                branch_bytes.next_power_of_two()
            };
            text = String::with_capacity(capacity);
            self.write_to(&mut text)
        };
        if result.is_err() {
            text.clear();
            text.push_str("ER_FMT_FAILED");
        }

        text
    }
}
impl fmt::Display for InternalReport<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_to(formatter)
    }
}
