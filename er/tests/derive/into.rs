use er::*;
use std::{cell::Cell, io, num::ParseIntError};

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
pub fn caller_fields() {
    let caller = String::from("preferences");
    let error = run()
        .er_into::<RequestError>(|tree| (85, tree.top.0.clone(), caller.as_str()))
        .unwrap_err();

    assert_eq!((error.request_id, error.label.as_str()), (85, "session"));
    assert!(core::ptr::eq(error.caller.as_ptr(), caller.as_ptr()));
    assert!(error.report.contains("task stopped"));

    let error: PairError = run().unwrap_err().er_into(|_| (85, 86));
    assert_eq!(error.1, (85, 86));
    assert!(error.0.er_report().to_string().contains("task stopped"));

    let calls = Cell::new(0);
    let result: Result<u8, RequestError> = Ok::<_, ErTree<RunErr>>(85).er_into(|_| {
        calls.set(calls.get() + 1);
        (85, "unused", caller.as_str())
    });
    assert_eq!(result.unwrap(), 85);
    assert_eq!(calls.get(), 0);

    #[cfg(target_has_atomic = "ptr")]
    {
        let error: Arc<RequestError> = run()
            .er_into(|_| (85, "session", caller.as_str()))
            .unwrap_err();
        assert_eq!(error.caller, caller);
    }
}

#[test]
pub fn conversion() {
    let mut observed = String::new();
    let calls = Cell::new(0);
    let result: Result<(), Saved<RunErr>> = run().er_into(|tree| {
        calls.set(calls.get() + 1);
        assert!(tree.er_find::<io::Error>().is_some());
        observed = tree.er_report().to_string();
    });

    let error = result.unwrap_err();
    assert_eq!(calls.get(), 1);
    assert_eq!(error.kind.0, "session");
    assert_eq!(error.to_string(), observed);

    let result: Result<u8, Saved<RunErr>> = Ok::<_, ErTree<RunErr>>(85).er_into(|tree| {
        calls.set(calls.get() + 1);
        observed = tree.er_report().to_string();
    });
    assert_eq!(result.unwrap(), 85);
    assert_eq!(calls.get(), 1);

    let snapshot: Result<(), SnapshotError> = run().er_report().er_into(|_| {});
    let snapshot = snapshot.unwrap_err();
    assert!(snapshot.to_string().contains("task stopped"));
    assert!(snapshot.to_string().contains('\n'));

    let wrapped = run().map_err(RunErrWrap::from);
    let saved: Result<(), Saved<RunErr>> = wrapped.er_into(|_| {});
    assert!(saved.unwrap_err().diagnostics.contains("task stopped"));

    let direct: Saved<RunErr> = run().unwrap_err().er_into(|tree| {
        assert!(tree.er_find::<io::Error>().is_some());
    });
    let report: Saved<RunErr> = run().unwrap_err().into_er_report().er_into(|_| {});
    let top: Saved<RunErr> = run().unwrap_err().into_er_top().er_into(|_| {});
    let wrap: Saved<RunErr> = RunErrWrap::from(run().unwrap_err()).er_into(|_| {});
    for error in [direct, report, top, wrap] {
        assert_eq!(error.kind.0, "session");
        assert!(error.diagnostics.contains("task stopped"));
    }

    #[cfg(target_has_atomic = "ptr")]
    {
        let shared: Result<(), Arc<Saved<RunErr>>> = run().er_into(|_| {});
        let reader = shared.clone();
        assert!(Arc::ptr_eq(&shared.unwrap_err(), &reader.unwrap_err()));
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
    let result: Result<(), PublicError> = run().er_into(|_| {});
    match result.unwrap_err() {
        PublicError::Run {
            kind,
            report,
            other_report,
            snapshot,
            other_snapshot,
        } => {
            assert_eq!(kind.0, "session");
            assert_eq!(report, other_report);
            assert_eq!(snapshot, other_snapshot);
            assert_eq!(report, snapshot.er_report().to_string());
        }
        _ => panic!("wrong variant"),
    }

    let parsed = "nope".parse::<u8>().map_err(ErTree::from);
    let result: Result<u8, PublicError> = parsed.er_into(|tree| {
        assert!(tree.top.to_string().contains("invalid digit"));
    });
    match result.unwrap_err() {
        PublicError::Parse(_, snapshot) => {
            let error = PublicError::parse("bad".parse::<u8>().unwrap_err(), snapshot);
            assert!(error.to_string().contains("invalid digit"));
            assert!(!error.to_string().contains("ErSnapshotEntry"));
        }
        _ => panic!("wrong variant"),
    }

    let plain = PublicError::missing();
    match plain {
        PublicError::Missing => (),
        _ => panic!("wrong variant"),
    }

    let result = Err::<(), _>(ErTree::from(io::Error::other("file gone")));
    let result: Result<(), PublicError> = result.er_into(|_| "settings.json");
    match result.unwrap_err() {
        PublicError::Io { path, kind, report } => {
            assert_eq!(path, "settings.json");
            assert_eq!(kind.to_string(), "file gone");
            assert!(report.contains("file gone"));
        }
        _ => panic!("wrong variant"),
    }
}
