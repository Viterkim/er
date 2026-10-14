use crate::{ErReport, ErSnapshot, Layout};
use alloc::string::String;
use core::error::Error;

// Keep the tags and field order compatible with the derive across Er versions.
macro_rules! saved_fields {
    ($tag:literal, $report:ident, $fields:ty, $body:block) => {
        impl<E: Error + 'static> From<ErReport<E>> for ($fields, [(); $tag]) {
            fn from($report: ErReport<E>) -> Self {
                let fields = $body;
                (fields, [(); $tag])
            }
        }
    };
}

saved_fields!(4, report, (String,), {
    let text = report.er_report_string();

    (text,)
});

saved_fields!(5, report, (E, String,), {
    let text = report.er_report_string();

    (report.tree.top, text)
});

saved_fields!(6, report, (String,), {
    let single = report.er_report().single_line().er_report_string();

    (single,)
});

saved_fields!(7, report, (E, String,), {
    let single = report.er_report().single_line().er_report_string();

    (report.tree.top, single)
});

saved_fields!(8, report, (String, String,), {
    let text = report.er_report_string();
    let single = match report.layout {
        Layout::SingleLine => text.clone(),
        Layout::Multiline => report.er_report().single_line().er_report_string(),
    };

    (text, single)
});

saved_fields!(9, report, (E, String, String,), {
    let text = report.er_report_string();
    let single = match report.layout {
        Layout::SingleLine => text.clone(),
        Layout::Multiline => report.er_report().single_line().er_report_string(),
    };

    (report.tree.top, text, single)
});

saved_fields!(10, report, (ErSnapshot,), {
    let snapshot = report.er_snapshot();

    (snapshot,)
});

saved_fields!(11, report, (E, ErSnapshot,), {
    let snapshot = report.er_snapshot();

    (report.tree.top, snapshot)
});

saved_fields!(12, report, (String, ErSnapshot,), {
    let text = report.er_report_string();
    let snapshot = report.er_snapshot();

    (text, snapshot)
});

saved_fields!(13, report, (E, String, ErSnapshot,), {
    let text = report.er_report_string();
    let snapshot = report.er_snapshot();

    (report.tree.top, text, snapshot)
});

saved_fields!(14, report, (String, ErSnapshot,), {
    let single = report.er_report().single_line().er_report_string();
    let snapshot = report.er_snapshot();

    (single, snapshot)
});

saved_fields!(15, report, (E, String, ErSnapshot,), {
    let single = report.er_report().single_line().er_report_string();
    let snapshot = report.er_snapshot();

    (report.tree.top, single, snapshot)
});

saved_fields!(16, report, (String, String, ErSnapshot,), {
    let text = report.er_report_string();
    let single = match report.layout {
        Layout::SingleLine => text.clone(),
        Layout::Multiline => report.er_report().single_line().er_report_string(),
    };
    let snapshot = report.er_snapshot();

    (text, single, snapshot)
});

saved_fields!(17, report, (E, String, String, ErSnapshot,), {
    let text = report.er_report_string();
    let single = match report.layout {
        Layout::SingleLine => text.clone(),
        Layout::Multiline => report.er_report().single_line().er_report_string(),
    };
    let snapshot = report.er_snapshot();

    (report.tree.top, text, single, snapshot)
});
