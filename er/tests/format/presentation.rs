#[cfg(feature = "src_locations")]
use core::panic::Location;
use core::{error::Error, fmt, ptr};
use er::*;

#[derive(Er)]
pub struct InnerErr;

#[derive(Er)]
pub struct AppErr;
pub fn failing() -> Er<(), AppErr> {
    Err(ErTree::new(AppErr, [InnerErr.er()]))
}

#[test]
pub fn views() {
    let error = failing().unwrap_err();
    let top = error.er_top();
    let report = error.er_report();

    assert_eq!(top.to_string(), "AppErr");
    assert!(report.to_string().contains("InnerErr"));
    assert_eq!(format!("{top}"), format!("{top:?}"));
    assert_eq!(format!("{report}"), format!("{report:?}"));
    assert!(top.source().is_none());
    assert!(report.source().is_none());

    assert_eq!(top.single_line().layout, Layout::SingleLine);
    assert_eq!(report.single_line().layout, Layout::SingleLine);
    assert_eq!(top.layout, Layout::Multiline);
    assert_eq!(report.layout, Layout::Multiline);

    let from_tree: ErReportRef<'_, _> = (&error).into();
    assert!(ptr::eq(from_tree.tree, report.tree));

    let from_tree: ErTopRef<'_, _> = (&error).into();
    assert!(ptr::eq(from_tree.tree, top.tree));

    let expected = report.to_string();

    let owned = error.into_er_report();
    assert!(owned.source().is_none());

    let owned: ErTop<_> = owned.tree.into();
    assert_eq!(owned.to_string(), "AppErr");
    assert_eq!(owned.layout, Layout::Multiline);

    let recovered = owned.tree;

    let owned: ErReport<_> = recovered.into();
    assert_eq!(owned.to_string(), expected);
    assert_eq!(owned.layout, Layout::Multiline);

    let boxed: Box<dyn Error + Send + Sync> = Box::new(owned);
    assert!(boxed.source().is_none());
}

#[test]
pub fn results() -> Result<(), ErReport<AppErr>> {
    pub fn propagate() -> Result<(), ErReport<AppErr>> {
        failing()?;

        Ok(())
    }

    assert!(propagate().unwrap_err().er_contains::<InnerErr>());

    let report = failing().er_report().unwrap_err();
    assert!(report.er_contains::<InnerErr>());
    assert_eq!(failing().er_top().unwrap_err().to_string(), "AppErr");

    let success: Er<u32, AppErr> = Ok(7);
    assert_eq!(success.er_report()?, 7);

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
        #[cfg(feature = "src_locations")]
        src_location: Location::caller(),
    };
    let top = tree.er_top();

    assert_eq!(format!("{:?}", top.single_line()), "first second");

    let mut lines = Vec::new();
    top.for_each_line(|line| lines.push(line.to_owned()))?;

    assert_eq!(lines, ["first", "second"]);
    assert_eq!(
        top.try_for_each_line(|_| Err(7)),
        Err(LineError::Callback(7))
    );

    let owned = tree.into_er_top();
    assert_eq!(format!("{owned:?}"), text);
    lines.clear();
    owned.for_each_line(|line| lines.push(line.to_owned()))?;

    assert_eq!(lines, ["first", "second"]);
    assert_eq!(
        owned.try_for_each_line(|_| Err(8)),
        Err(LineError::Callback(8))
    );

    Ok(())
}
