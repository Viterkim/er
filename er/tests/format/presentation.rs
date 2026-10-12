#[cfg(feature = "src_locations")]
use core::panic::Location;
use core::{error::Error, fmt, ptr};
use er::*;

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
pub fn views() -> fmt::Result {
    let error = failing().unwrap_err();
    let top = error.er_top();
    let report = error.er_report();

    assert_eq!(top.to_string(), "AppErr");
    assert!(report.to_string().contains("InnerErr"));
    assert_eq!(format!("{top}"), format!("{top:?}"));
    assert_eq!(format!("{report}"), format!("{report:?}"));
    assert!(Error::source(&top.opaque_err()).is_none());
    assert!(Error::source(&report.opaque_err()).is_none());

    assert_eq!(top.single_line().layout, Layout::SingleLine);
    assert_eq!(report.single_line().layout, Layout::SingleLine);
    assert_eq!(top.layout, Layout::Multiline);
    assert_eq!(report.layout, Layout::Multiline);

    assert_eq!(top.single_line().er_report().layout, Layout::SingleLine);
    assert_eq!(report.single_line().er_top().layout, Layout::SingleLine);

    let from_tree: ErReportRef<'_, _> = (&error).into();
    assert!(ptr::eq(from_tree.tree, report.tree));

    let from_tree: ErTopRef<'_, _> = (&error).into();
    assert!(ptr::eq(from_tree.tree, top.tree));

    let expected = report.to_string();
    assert_eq!(error.er_report_string(), expected);
    assert_eq!(error.er_top_string(), "AppErr");
    assert_eq!(
        report.single_line().er_report_string(),
        report.single_line().to_string()
    );
    assert_eq!(
        top.single_line().er_report_string(),
        report.single_line().to_string()
    );

    let owned = error.into_er_report();
    assert_eq!(owned.er_report().layout, Layout::Multiline);
    assert_eq!(owned.er_top().layout, Layout::Multiline);

    let owned: ErTop<_> = owned.tree.into();
    assert_eq!(owned.er_top().layout, Layout::Multiline);
    assert_eq!(owned.er_report().layout, Layout::Multiline);
    assert_eq!(owned.to_string(), "AppErr");
    assert_eq!(owned.layout, Layout::Multiline);

    let recovered = owned.tree;

    let owned: ErReport<_> = recovered.into();
    assert_eq!(owned.to_string(), expected);
    assert_eq!(owned.layout, Layout::Multiline);

    let error = owned.opaque_err();
    assert_eq!(error.to_string(), expected);
    assert_eq!(format!("{error:?}"), expected);
    assert!(error.source().is_none());
    let outer = Err::<(), _>(error)
        .er(OuterErr::new)
        .err()
        .ok_or(fmt::Error)?;
    assert_eq!(outer.nodes[0].error.to_string(), expected);
    let adapter = outer
        .er_find::<ErAsError<ErReport<AppErr>>>()
        .ok_or(fmt::Error)?;
    assert!(adapter.0.er_contains::<InnerErr>());
    Ok(())
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

    fn local_opaque(tree: &ErTree<AppErr>) -> ErAsError<ErReportRef<'_, AppErr>> {
        let mut result = Err::<(), _>(tree.er_report().single_line());
        result.as_mut().opaque_err().unwrap_err()
    }

    fn local_boxed(tree: &ErTree<AppErr>) -> ErReportRef<'_, AppErr> {
        let boxed = Box::new(tree.er_top().single_line());
        Err::<(), _>(&boxed).er_report().unwrap_err()
    }

    fn check<'a, P: Clone + IntoErTreeRef<'a, Error = AppErr>>(
        error: P,
        tree: &ErTree<AppErr>,
        expected: &str,
    ) {
        assert_eq!(
            Err::<(), _>(error.clone()).er_report_string().unwrap_err(),
            expected
        );
        assert_eq!(
            Err::<(), _>(error.clone()).er_top_string().unwrap_err(),
            "AppErr"
        );
        let report = Err::<(), _>(error.clone())
            .er_top()
            .er_report()
            .unwrap_err();
        assert_eq!(report.er_report_string(), expected);
        assert!(ptr::eq(report.tree, tree));
        assert!(ptr::eq(
            Err::<(), _>(error.clone()).er_tree().unwrap_err(),
            tree
        ));
        assert_eq!(
            Err::<(), _>(error).er_snapshot().unwrap_err(),
            tree.er_snapshot()
        );

        assert_eq!(Ok::<_, P>(85).unwrap_report(), 85);
        assert_eq!(Ok::<_, P>(85).expect_report("failure"), 85);
    }

    let tree = failing().unwrap_err();
    check(&tree, &tree, &tree.er_report_string());

    for layout in [Layout::Multiline, Layout::SingleLine] {
        let top = tree.er_top().layout(layout);
        let report = tree.er_report().layout(layout);
        let expected = report.er_report_string();
        check(top, &tree, &expected);
        check::<&ErTopRef<'_, _>>(&top, &tree, &expected);
        check(report, &tree, &expected);
        check::<&ErReportRef<'_, _>>(&report, &tree, &expected);

        let report: ErReportRef<'_, _> = propagate(Err::<(), _>(top)).unwrap_err();
        let top: ErTopRef<'_, _> = propagate(Err::<(), _>(report)).unwrap_err();
        assert!(ptr::eq(top.tree, &tree));
        assert_eq!(top.layout, layout);
    }

    let mut result = failing();
    let snapshot = result.as_ref().er_snapshot().unwrap_err();
    assert_eq!(result.as_mut().er_snapshot().unwrap_err(), snapshot);

    let tree = result.as_mut().er_tree().unwrap_err();
    assert!(ptr::eq(local_view(tree).tree, tree));
    assert!(ptr::eq(local_opaque(tree).0.tree, tree));
    assert!(ptr::eq(local_boxed(tree).tree, tree));

    let result = result.er_report();
    let mut result = Err::<(), _>(result.unwrap_err().single_line());
    let expected = result.as_ref().unwrap_err().er_report_string();
    let report = result.as_ref().unwrap_err();
    check(report, &report.tree, &expected);
    assert_eq!(result.as_ref().er_report_string().unwrap_err(), expected);
    assert_eq!(result.as_mut().er_report_string().unwrap_err(), expected);
    assert_eq!(
        result.as_ref().opaque_err().unwrap_err().to_string(),
        expected
    );
    assert_eq!(
        result.as_mut().opaque_err().unwrap_err().to_string(),
        expected
    );

    let mut result = result.er_top();
    assert_eq!(result.as_ref().er_report_string().unwrap_err(), expected);
    assert_eq!(
        result.as_mut().er_report().unwrap_err().er_report_string(),
        expected
    );

    let opaque = result.as_mut().opaque_err().unwrap_err();
    assert_eq!(opaque.to_string(), "AppErr");
    assert_eq!(opaque.0.layout, Layout::SingleLine);

    let mut success = Ok::<_, ErReport<AppErr>>(String::from("kept"));
    assert!(ptr::eq(
        success.as_ref().unwrap_report(),
        success.as_ref().unwrap()
    ));
    success.as_mut().unwrap_report().push_str(" value");
    assert_eq!(success.unwrap_report(), "kept value");

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        result.as_ref().expect_report("failure")
    }))
    .unwrap_err();
    let message = panic.downcast::<String>().unwrap();
    assert!(message.contains(&expected));
    assert!(message.contains("failure"));
}

#[test]
pub fn reentry() {
    let report = failing().er_report().unwrap_err().single_line();
    let outer = Err::<(), _>(report).er(OuterErr::new).unwrap_err();
    assert!(outer.er_contains::<AppErr>());
    assert!(outer.er_contains::<InnerErr>());
    assert_eq!(outer.into_er_report().layout, Layout::Multiline);

    let top = failing().er_top().unwrap_err().single_line();
    let outer = Err::<(), _>(top).er(OuterErr::new).unwrap_err();
    assert!(outer.er_contains::<AppErr>());
    assert!(outer.er_contains::<InnerErr>());
    assert_eq!(outer.into_er_report().layout, Layout::Multiline);

    for outer in [
        failing()
            .er_report()
            .unwrap_err()
            .single_line()
            .er(OuterErr::new),
        failing()
            .er_top()
            .unwrap_err()
            .single_line()
            .er(OuterErr::new),
    ] {
        assert!(outer.er_contains::<AppErr>());
        assert!(outer.er_contains::<InnerErr>());
        assert_eq!(outer.into_er_report().layout, Layout::Multiline);
    }
}

#[test]
pub fn results() -> Result<(), ErReport<AppErr>> {
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

    let report = failing().er_report().unwrap_err();
    assert!(report.er_contains::<InnerErr>());
    assert_eq!(failing().er_top().unwrap_err().to_string(), "AppErr");

    let success: ErResult<u32, AppErr> = Ok(7);
    assert_eq!(success.er_report()?, 7);

    assert_eq!(
        Ok::<_, ErTree<AppErr>>(String::from("kept")).er_report_string(),
        Ok(String::from("kept"))
    );
    assert_eq!(Ok::<_, ErTree<AppErr>>(7).er_top_string(), Ok(7));
    assert_eq!(Ok::<_, ErTree<AppErr>>(7).er_snapshot().unwrap(), 7);

    let expected = failing().unwrap_err().er_report_string();
    assert_eq!(failing().er_report_string().unwrap_err(), expected);
    assert_eq!(failing().er_top_string().unwrap_err(), "AppErr");
    assert_eq!(
        failing().er_snapshot().unwrap_err().er_report_string(),
        expected
    );

    let report = failing().er_report().unwrap_err().single_line();
    let expected = report.er_report_string();
    assert_eq!(
        Err::<(), _>(report).er_report_string().unwrap_err(),
        expected
    );

    let report = failing().er_report().unwrap_err().single_line();
    let nodes = report.tree.nodes.as_ptr();
    let top = to_top(Err::<(), _>(report)).unwrap_err();
    assert_eq!(top.layout, Layout::SingleLine);
    assert_eq!(top.er_report_string(), expected);
    assert_eq!(top.tree.nodes.as_ptr(), nodes);
    let report = to_report(Err::<(), _>(top)).unwrap_err();
    assert_eq!(report.layout, Layout::SingleLine);
    assert_eq!(report.tree.nodes.as_ptr(), nodes);
    assert_eq!(report.er_report_string(), expected);

    let top = failing().er_top().unwrap_err().single_line();
    let report = Err::<(), _>(top).er_report().unwrap_err();
    assert_eq!(report.er_report_string(), expected);

    let nodes = report.tree.nodes.as_ptr();
    let mut boxed = Err::<(), _>(Box::new(report));
    assert_eq!(boxed.as_ref().er_report_string().unwrap_err(), expected);
    assert_eq!(boxed.as_mut().er_report_string().unwrap_err(), expected);
    assert_eq!(
        boxed.as_mut().opaque_err().unwrap_err().to_string(),
        expected
    );

    let report = boxed.er_top().er_report().unwrap_err();
    assert_eq!(report.tree.nodes.as_ptr(), nodes);
    assert_eq!(report.layout, Layout::SingleLine);

    Ok(())
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

    assert_eq!(tree.er_report().er_top_string(), text);
    assert_eq!(
        Err::<(), _>(tree.er_report()).er_top_string().unwrap_err(),
        text
    );

    assert_eq!(format!("{:?}", top.single_line()), "first second");
    let adapter = top.single_line().opaque_err();
    assert_eq!(adapter.to_string(), "first second");
    assert!(Error::source(&adapter).is_none());

    let mut lines = Vec::new();
    top.for_each_line(|line| lines.push(line.to_owned()))?;

    assert_eq!(lines, ["first", "second"]);
    assert_eq!(
        top.try_for_each_line(|_| Err(7)),
        Err(ErLineError::Callback(7))
    );

    let owned = tree.into_er_top();
    assert_eq!(format!("{owned:?}"), text);
    lines.clear();
    owned.for_each_line(|line| lines.push(line.to_owned()))?;

    assert_eq!(lines, ["first", "second"]);
    assert_eq!(
        owned.try_for_each_line(|_| Err(8)),
        Err(ErLineError::Callback(8))
    );

    let report = owned.into_er_report().single_line();
    let expected = report.er_top_string();
    assert_eq!(expected, "first second");
    assert_eq!(Err::<(), _>(report).er_top_string().unwrap_err(), expected);

    Ok(())
}
