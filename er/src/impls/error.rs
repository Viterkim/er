use crate::render::write_report;
use crate::{ErEntries, ErErrorPresentationExt, ErSnapshot, Layout};
use alloc::string::{String, ToString};
use core::error::Error;

/// Print an error and its native sources.
pub fn report_string(error: &(dyn Error + 'static)) -> String {
    let mut report = String::new();
    if write_report(&mut report, error, &[], Layout::Multiline, None).is_err() {
        report.clear();
        report.push_str("ER_FMT_FAILED");
    }
    report
}

impl<E: Error> ErErrorPresentationExt for E {
    fn er_report_string(&self) -> String
    where
        Self: 'static,
    {
        report_string(self)
    }

    fn er_top_string(&self) -> String {
        self.to_string()
    }

    fn er_snapshot(&self) -> ErSnapshot
    where
        Self: 'static,
    {
        ErEntries::new(self, &[], None).into()
    }
}
impl ErErrorPresentationExt for dyn Error + 'static {
    fn er_report_string(&self) -> String {
        report_string(self)
    }

    fn er_top_string(&self) -> String {
        self.to_string()
    }

    fn er_snapshot(&self) -> ErSnapshot {
        ErEntries::new(self, &[], None).into()
    }
}
impl ErErrorPresentationExt for dyn Error + Send + Sync + 'static {
    fn er_report_string(&self) -> String {
        report_string(self)
    }

    fn er_top_string(&self) -> String {
        self.to_string()
    }

    fn er_snapshot(&self) -> ErSnapshot {
        ErEntries::new(self, &[], None).into()
    }
}
impl ErErrorPresentationExt for dyn Error + Send + 'static {
    fn er_report_string(&self) -> String {
        report_string(self)
    }

    fn er_top_string(&self) -> String {
        self.to_string()
    }

    fn er_snapshot(&self) -> ErSnapshot {
        ErEntries::new(self, &[], None).into()
    }
}
impl ErErrorPresentationExt for dyn Error + Sync + 'static {
    fn er_report_string(&self) -> String {
        report_string(self)
    }

    fn er_top_string(&self) -> String {
        self.to_string()
    }

    fn er_snapshot(&self) -> ErSnapshot {
        ErEntries::new(self, &[], None).into()
    }
}
