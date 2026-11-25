use crate::render::write_report;
use crate::{ErBoxedInput, ErEntries, ErErrorPresentationExt, ErReportError, ErSnapshot};
use alloc::boxed::Box;
use alloc::string::String;
use core::{error::Error, fmt};

/// Print an error and its native sources.
pub fn report_string(error: &(dyn Error + 'static)) -> String {
    let mut report = String::with_capacity(32);
    if write_report(
        &mut report as &mut dyn fmt::Write,
        error,
        &[],
        &[],
        &[],
        None,
    )
    .is_err()
    {
        report.clear();
        report.push_str("ER_FMT_FAILED");
    }
    report
}

pub fn write_error_report(
    error: &(dyn Error + 'static),
    writer: &mut (impl fmt::Write + ?Sized),
) -> fmt::Result {
    write_report(writer, error, &[], &[], &[], None)
}

impl<E: Error> ErErrorPresentationExt for E {
    fn er_report(&self) -> String
    where
        Self: 'static,
    {
        report_string(self)
    }

    fn er_write_report(&self, writer: &mut (impl fmt::Write + ?Sized)) -> fmt::Result
    where
        Self: 'static,
    {
        write_error_report(self, writer)
    }

    fn er_snapshot(&self) -> ErSnapshot
    where
        Self: 'static,
    {
        ErEntries::new(self, &[], None).into()
    }
}
macro_rules! dyn_error {
    ($($bounds:tt)*) => {
        impl ErErrorPresentationExt for dyn Error $($bounds)* {
            fn er_report(&self) -> String {
                report_string(self)
            }

            fn er_write_report(&self, writer: &mut (impl fmt::Write + ?Sized)) -> fmt::Result {
                write_error_report(self, writer)
            }

            fn er_snapshot(&self) -> ErSnapshot {
                ErEntries::new(self, &[], None).into()
            }
        }
    };
}
dyn_error!(+ 'static);
dyn_error!(+ Send + Sync + 'static);
dyn_error!(+ Send + 'static);
dyn_error!(+ Sync + 'static);

impl<E: Error + 'static> ErReportError<()> for E {
    fn as_error(&self) -> &(dyn Error + 'static) {
        self
    }
}

macro_rules! boxed_report_error {
    ($($bounds:tt)*) => {
        impl ErReportError<ErBoxedInput> for Box<dyn Error $($bounds)*> {
            fn as_error(&self) -> &(dyn Error + 'static) {
                &**self
            }
        }

        impl ErErrorPresentationExt<ErBoxedInput> for Box<dyn Error $($bounds)*> {
            fn er_report(&self) -> String {
                report_string(self.as_error())
            }

            fn er_write_report(&self, writer: &mut (impl fmt::Write + ?Sized)) -> fmt::Result {
                write_error_report(self.as_error(), writer)
            }

            fn er_snapshot(&self) -> ErSnapshot {
                ErEntries::new(self.as_error(), &[], None).into()
            }
        }
    };
}
boxed_report_error!();
boxed_report_error!(+ Send + Sync);
boxed_report_error!(+ Send);
boxed_report_error!(+ Sync);
