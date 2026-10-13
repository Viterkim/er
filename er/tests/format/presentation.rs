use core::fmt;
#[cfg(feature = "src_locations")]
use core::panic::Location;
use er::*;
use std::{rc::Rc, sync::Arc};

#[derive(Er)]
pub struct InnerErr;

#[derive(Er)]
pub struct OuterErr;

#[derive(Er)]
pub struct AppErr;
pub fn failing() -> ErResult<(), AppErr> {
    Err(ErTree::new(AppErr, [ErTree::from(InnerErr)]))
}

#[test]
pub fn views() {
    let error = failing().unwrap_err();
    let top = error.er_top();
    let report = error.er_report();

    assert_eq!(top.to_string(), "AppErr");
    assert!(report.to_string().contains("InnerErr"));
    assert_eq!(format!("{report}"), format!("{report:?}"));

    let _: ErReportRef<'_, _> = (&error).into();
    let _: ErTopRef<'_, _> = (&error).into();
    assert_eq!(
        top.single_line().er_report_string(),
        report.single_line().to_string()
    );
}

#[test]
pub fn borrowed_results() {
    fn propagate<T, E, F: From<E>>(result: Result<T, E>) -> Result<T, F> {
        Ok(result?)
    }

    fn local_view(tree: &ErTree<AppErr>) -> ErReportRef<'_, AppErr> {
        let mut result = Err::<(), _>(tree.er_top().single_line());
        result.as_mut().er_report().unwrap_err()
    }

    fn local_report(tree: &ErTree<AppErr>) -> ErReportRef<'_, AppErr> {
        let mut result = Err::<(), _>(tree.er_report().single_line());
        result.as_mut().er_report().unwrap_err()
    }

    fn local_boxed(tree: &ErTree<AppErr>) -> ErReportRef<'_, AppErr> {
        let boxed = Box::new(tree.er_top().single_line());
        Err::<(), _>(&boxed).er_report().unwrap_err()
    }

    fn check<'a, P: Clone + IntoErTreeRef<'a, Error = AppErr>>(error: P, expected: &str) {
        let report = Err::<(), _>(error.clone()).er_top().er_report();
        assert_eq!(report.unwrap_err().er_report_string(), expected);
        assert_eq!(
            Err::<(), _>(error).er_report_string().unwrap_err(),
            expected
        );
    }

    let tree = failing().unwrap_err();
    check(&tree, &tree.er_report_string());

    for layout in [Layout::Multiline, Layout::SingleLine] {
        let top = tree.er_top().layout(layout);
        let report = tree.er_report().layout(layout);
        let expected = report.er_report_string();
        check(top, &expected);
        check::<&ErTopRef<'_, _>>(&top, &expected);
        check(report, &expected);
        check::<&ErReportRef<'_, _>>(&report, &expected);
        check(&Arc::new(report), &expected);
        check(&Rc::new(top), &expected);

        let mut borrowed = Err::<(), _>(&report);
        let view = borrowed.as_mut().er_report().unwrap_err();
        assert_eq!(view.to_string(), expected);

        let report: ErReportRef<'_, _> = propagate(Err::<(), _>(top)).unwrap_err();
        let top: ErTopRef<'_, _> = propagate(Err::<(), _>(report)).unwrap_err();
        assert_eq!(top.er_report_string(), expected);
    }

    let mut result = failing();
    let tree = result.as_mut().er_tree().unwrap_err();
    let expected = tree.er_report().single_line().to_string();
    for view in [local_view(tree), local_report(tree), local_boxed(tree)] {
        assert_eq!(view.to_string(), expected);
    }

    let result = Err::<(), _>(result.er_report().unwrap_err().single_line());
    check(result.as_ref().unwrap_err(), &expected);

    let mut result = result.er_top();
    let view = result.as_mut().er_top().unwrap_err();
    assert_eq!(view.to_string(), "AppErr");

    let mut success = Ok::<_, ErReport<AppErr>>(String::from("kept"));
    success.as_mut().unwrap_report().push_str(" value");
    assert_eq!(success.unwrap_report(), "kept value");
}

#[test]
pub fn reentry() {
    let report = || failing().er_report().unwrap_err().single_line();
    let top = || failing().er_top().unwrap_err().single_line();

    let outer = Err::<(), _>(report()).er(OuterErr::new).unwrap_err();
    assert_eq!(outer.er_report().layout, Layout::Multiline);

    for outer in [
        outer,
        Err::<(), _>(top()).er(OuterErr::new).unwrap_err(),
        report().er(OuterErr::new),
        top().er(OuterErr::new),
    ] {
        assert!(outer.er_contains::<InnerErr>());
    }
}

#[test]
pub fn results() {
    fn to_report<T, E>(result: Result<T, ErTop<E>>) -> Result<T, ErReport<E>> {
        Ok(result?)
    }

    fn to_top<T, E>(result: Result<T, ErReport<E>>) -> Result<T, ErTop<E>> {
        Ok(result?)
    }

    pub fn propagate() -> Result<(), ErReport<AppErr>> {
        failing()?;

        Ok(())
    }

    assert!(propagate().unwrap_err().er_contains::<InnerErr>());

    let expected = failing().unwrap_err().er_report_string();
    assert_eq!(failing().er_report_string().unwrap_err(), expected);

    let report = failing().er_report().unwrap_err().single_line();
    let expected = report.er_report_string();
    let top = to_top(Err::<(), _>(report)).unwrap_err();
    let report = to_report(Err::<(), _>(top)).unwrap_err();
    assert_eq!(report.er_report_string(), expected);

    let boxed = Err::<(), _>(Box::new(report));
    let report = boxed.er_top().er_report().unwrap_err();
    assert_eq!(report.er_report_string(), expected);
}

pub struct Text<'a>(pub &'a str);
impl fmt::Display for Text<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

#[test]
pub fn display_only() -> fmt::Result {
    let text = String::from("first\nsecond");
    let tree = ErTree {
        top: Text(&text),
        nodes: Vec::new(),
        #[cfg(feature = "stack_traces")]
        stack_traces: Vec::new(),
        #[cfg(feature = "src_locations")]
        src_location: Location::caller(),
    };
    let top = tree.er_top();
    assert_eq!(format!("{:?}", top.single_line()), "first second");

    let mut lines = Vec::new();
    top.for_each_line(|l| lines.push(l.to_owned()))?;
    assert_eq!(lines, ["first", "second"]);

    assert_eq!(
        Err::<(), _>(tree.er_report()).er_top_string().unwrap_err(),
        text
    );

    Ok(())
}
