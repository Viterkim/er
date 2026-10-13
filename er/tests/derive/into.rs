use er::*;
use std::{io, num::ParseIntError};

#[cfg(target_has_atomic = "ptr")]
use std::sync::Arc;

#[derive(Er)]
#[er(wrap(output = report, std_error))]
pub struct RunErr(pub String);
pub fn run() -> ErResult<(), RunErr> {
    Err::<(), _>(io::Error::other("task stopped")).er(|_| "session")
}

#[derive(Er)]
#[er(format = "{diagnostics}")]
pub struct Saved<E> {
    #[er(into_top)]
    pub kind: E,
    #[er(into_report_string)]
    pub diagnostics: String,
    #[er(into_report_string(single_line))]
    pub single_line: String,
}

#[derive(Er)]
#[er(format = "{diagnostics}")]
pub struct SnapshotError {
    #[er(into_snapshot)]
    pub diagnostics: ErSnapshot,
}

#[derive(Er)]
pub struct RequestError<'a> {
    pub request_id: u32,
    #[er(into_report_string)]
    pub report: String,
    pub label: String,
    #[er(into_top)]
    pub kind: RunErr,
    pub caller: &'a str,
}

#[derive(Er)]
pub struct PairError(#[er(into_snapshot)] pub ErSnapshot, pub (u8, u8));

#[test]
pub fn fields() {
    #[derive(Er)]
    struct PublicError {
        diagnostics: String,
        request: String,
        kind: RunErr,
    }

    fn convert(request: &str) -> Result<(), PublicError> {
        run().er_into(|report| (report.er_report_string(), request, report.tree.top))
    }

    fn extra(request: &str) -> Result<(), RequestError<'_>> {
        run().er_into(|report| (report, (85, "session", request)))
    }

    fn foreign() -> Result<(), io::Error> {
        run().er_into(|report| er_built(io::Error::other(report.er_report_string())))
    }

    let err = convert("preferences").unwrap_err();
    assert!(err.diagnostics.contains("task stopped"));

    assert_eq!(extra("preferences").unwrap_err().caller, "preferences");
    assert!(foreign().unwrap_err().to_string().contains("task stopped"));
}

#[test]
pub fn caller_fields() {
    let caller = String::from("preferences");
    let error: RequestError = run()
        .er_into(|report| {
            let fields = (85, report.tree.top.0.clone(), caller.as_str());
            (report, fields)
        })
        .unwrap_err();

    assert_eq!(error.label, "session");

    let error: PairError = run().unwrap_err().er_into(|report| (report, (85, 86)));
    assert_eq!(error.1, (85, 86));

    let _: Box<RequestError> = run()
        .er_into(|report| (report, (85, "session", caller.as_str())))
        .unwrap_err();

    #[cfg(target_has_atomic = "ptr")]
    let _: Arc<RequestError> = run()
        .er_into(|report| (report, (85, "session", caller.as_str())))
        .unwrap_err();
}

#[test]
pub fn layouts() {
    let tree = run().unwrap_err();
    let multiline = tree.er_report_string();
    let single_line = tree.er_report().single_line().to_string();
    let error: Saved<RunErr> = tree.er_into(|report| report);

    assert_eq!(error.diagnostics, multiline);
    assert_eq!(error.single_line, single_line);

    let report = run().unwrap_err().into_er_report().single_line();
    let expected = report.er_report_string();
    let error: Saved<RunErr> = report.er_into(|report| report);
    assert_eq!(error.diagnostics, expected);

    let top = run().unwrap_err().into_er_top().single_line();
    let expected = top.er_report_string();
    let error: Box<Saved<RunErr>> = Err::<(), _>(top).er_into(|report| report).unwrap_err();
    assert_eq!(error.diagnostics, expected);

    #[cfg(target_has_atomic = "ptr")]
    let _: Arc<Saved<RunErr>> = run().er_into(|report| report).unwrap_err();
}

#[test]
pub fn conversion() {
    let snapshot: Result<(), SnapshotError> = run().er_report().er_into(|report| report);
    assert!(snapshot.unwrap_err().to_string().contains("task stopped"));

    let direct: Saved<RunErr> = run().unwrap_err().er_into(|report| report);
    let report: Saved<RunErr> = run().unwrap_err().into_er_report().er_into(|report| report);
    let top: Saved<RunErr> = run().unwrap_err().into_er_top().er_into(|report| report);
    let wrap: Saved<RunErr> = RunErrWrap::from(run().unwrap_err()).er_into(|report| report);
    for error in [direct, report, top, wrap] {
        assert_eq!(error.kind.0, "session");
    }
}

#[derive(Er)]
pub enum PublicError {
    Missing,
    Run {
        #[er(into_top)]
        kind: RunErr,
        #[er(into_report_string)]
        report: String,
        #[er(into_report_string)]
        other_report: String,
        #[er(into_report_string(single_line))]
        single_line: String,
        #[er(into_report_string(single_line))]
        other_single_line: String,
        #[er(into_snapshot)]
        snapshot: ErSnapshot,
        #[er(into_snapshot)]
        other_snapshot: ErSnapshot,
    },
    Parse(
        #[er(into_top)] ParseIntError,
        #[er(into_snapshot)] ErSnapshot,
    ),
    Io {
        path: String,
        #[er(into_top)]
        kind: io::Error,
        #[er(into_report_string)]
        report: String,
    },
}
#[test]
pub fn variants() {
    let result: Result<(), PublicError> = run().er_into(|report| report);
    match result.unwrap_err() {
        PublicError::Run {
            report,
            other_report,
            single_line,
            other_single_line,
            snapshot,
            other_snapshot,
            ..
        } => {
            assert_eq!(report, other_report);
            assert_eq!(single_line, other_single_line);
            assert_eq!(snapshot, other_snapshot);
        }
        _ => panic!("wrong variant"),
    }

    let parsed = "nope".parse::<u8>().map_err(ErTree::from);
    let result: Result<u8, PublicError> = parsed.er_into(|report| report);
    match result.unwrap_err() {
        PublicError::Parse(_, snapshot) => {
            let error = PublicError::parse("bad".parse::<u8>().unwrap_err(), snapshot);
            assert!(error.to_string().contains("invalid digit"));
            assert!(!error.to_string().contains("ErSnapshotEntry"));
        }
        _ => panic!("wrong variant"),
    }

    let result = Err::<(), _>(ErTree::from(io::Error::other("file gone")));
    let result: Result<(), PublicError> = result.er_into(|report| (report, "settings.json"));
    match result.unwrap_err() {
        PublicError::Io { kind, .. } => assert_eq!(kind.to_string(), "file gone"),
        _ => panic!("wrong variant"),
    }
}

pub mod type_macros {
    use er::*;

    #[derive(Er)]
    pub struct Top<const LINE: u32>;

    macro_rules! selected {
        () => { Top<{ line!() }> };
    }

    #[derive(Er)]
    pub enum Boundary {
        First {
            #[er(into_top)]
            top: selected!(),
            #[er(into_report_string)]
            report: String,
        },
        Second {
            #[er(into_top)]
            top: selected!(),
            #[er(into_report_string)]
            report: String,
        },
    }

    #[test]
    pub fn distinct_types() {
        let first = Boundary::first(Top::new(), "first");
        let second = Boundary::second(Top::new(), "second");

        for (error, is_first) in [(first, true), (second, false)] {
            let converted: Boundary = match error {
                Boundary::First { top, .. } => ErTree::from(top).er_into(|report| report),
                Boundary::Second { top, .. } => ErTree::from(top).er_into(|report| report),
            };

            let converted_first = match converted {
                Boundary::First { .. } => true,
                Boundary::Second { .. } => false,
            };
            assert_eq!(converted_first, is_first);
        }
    }
}
