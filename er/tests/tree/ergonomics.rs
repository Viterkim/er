use er::*;
use std::cell::Cell;

#[derive(Er)]
pub struct AppErr;
pub fn plain_result() -> Result<(), AppErr> {
    Err(AppErr)
}

pub fn converted_with_question_mark(expected_line: &mut u32) -> Er<(), AppErr> {
    *expected_line = line!() + 1;
    plain_result()?;

    Ok(())
}

pub fn reject_if_missing(missing: bool, expected_line: &mut u32) -> Er<(), AppErr> {
    if missing {
        *expected_line = line!() + 1;
        return Err(AppErr.er());
    }

    Ok(())
}

#[test]
pub fn conversion() {
    let _line = line!() + 1;
    let tree: ErTree<AppErr> = AppErr.into();

    assert!(tree.nodes.is_empty());
    #[cfg(feature = "src_locations")]
    assert_eq!(tree.src_location.line(), _line);
}

#[test]
pub fn question_mark() {
    let mut expected_line = 0;
    let error = converted_with_question_mark(&mut expected_line).unwrap_err();

    assert!(error.nodes.is_empty());
    assert_eq!(error.top.to_string(), "AppErr");
    #[cfg(feature = "src_locations")]
    assert_eq!(error.src_location.line(), expected_line);
}

#[test]
pub fn early_return() {
    let mut expected_line = 0;
    assert!(reject_if_missing(false, &mut expected_line).is_ok());

    let error = reject_if_missing(true, &mut expected_line).unwrap_err();
    assert!(error.nodes.is_empty());
    assert_eq!(error.top.to_string(), "AppErr");
    #[cfg(feature = "src_locations")]
    assert_eq!(error.src_location.line(), expected_line);
}

#[derive(Er)]
pub struct ChildErr(pub u8);
#[test]
pub fn er_from() -> Result<(), ErReport<ChildErr>> {
    let calls = Cell::new(0);
    let input: Result<u8, u8> = Ok(7);
    let result: Er<u8, ChildErr> = input.er_from(|value| {
        calls.set(calls.get() + 1);
        ChildErr(value)
    });

    assert_eq!(result.er_report()?, 7);
    assert_eq!(calls.get(), 0);

    let result: Result<(), u8> = Err(7);
    let _expected_line = line!() + 1;
    let error = result.er_from(ChildErr).unwrap_err();

    assert_eq!(error.top.0, 7);
    assert!(error.nodes.is_empty());
    #[cfg(feature = "src_locations")]
    assert_eq!(error.src_location.file(), file!());
    #[cfg(feature = "src_locations")]
    assert_eq!(error.src_location.line(), _expected_line);

    Ok(())
}

#[derive(Er)]
pub struct LocalErr {
    pub attempts: Cell<u8>,
    pub label: std::rc::Rc<str>,
}
pub fn local_error() -> LocalErr {
    LocalErr::new(Cell::new(7), "local")
}

#[test]
pub fn local_roots() {
    let root = local_error().er();
    assert_eq!(root.top.attempts.get(), 7);
    assert!(root.er_report().to_string().contains("local"));
    assert!(root.er_find::<LocalErr>().is_some());

    let converted = ErTree::from(local_error());
    assert!(converted.nodes.is_empty());

    let missing: Option<()> = None;
    assert!(missing.er(local_error).unwrap_err().nodes.is_empty());

    let status: Result<(), u8> = Err(7);
    let mapped = status.er_from(|attempts| LocalErr::new(Cell::new(attempts), "mapped"));

    assert_eq!(mapped.unwrap_err().top.attempts.get(), 7);

    let result: Result<(), ChildErr> = Err(ChildErr(1));
    let context = result.er(local_error).unwrap_err();

    assert_eq!(context.er_find::<ChildErr>().unwrap().0, 1);

    let context = ChildErr(2).er().er(local_error);
    assert_eq!(context.er_find::<ChildErr>().unwrap().0, 2);

    let grouped = ErTree::new(local_error(), [ChildErr(3)]);
    assert_eq!(grouped.er_find::<ChildErr>().unwrap().0, 3);

    let result: Result<(), ChildErr> = Err(ChildErr(4));
    let results = vec![result];
    let collected = er_all!(local_error, results).unwrap_err();

    assert_eq!(collected.er_find::<ChildErr>().unwrap().0, 4);

    let result: Result<(), ChildErr> = Err(ChildErr(5));
    let collected = er_all!(local_error, [result]).unwrap_err();

    assert_eq!(collected.er_find::<ChildErr>().unwrap().0, 5);
}
