use er::*;
use std::io;

#[derive(Er)]
#[er(wrap(output = report, std_error))]
pub struct RunErr(pub String);
pub fn run() -> ErResult<(), RunErr> {
    Err::<(), _>(io::Error::other("task stopped")).er(|_| "session")
}

#[test]
pub fn closure() {
    let error = run()
        .er_split_with(|(_, report)| io::Error::other(report))
        .unwrap_err();

    assert!(error.to_string().contains("task stopped"));
}

#[test]
pub fn conversion() {
    let tree = run().unwrap_err().er_split();
    let wrap = RunErrWrap::from(run().unwrap_err()).er_split();

    for (top, report) in [tree, wrap] {
        assert_eq!(top.0, "session");
        assert!(report.contains("task stopped"));
    }
}
