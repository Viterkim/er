use er::*;
use std::{
    any::{Any, TypeId},
    cell::Cell,
};

#[derive(Er)]
pub struct AppErr;
pub fn plain_result() -> Result<(), AppErr> {
    Err(AppErr)
}

pub fn converted_with_question_mark(expected_line: &mut u32) -> ErResult<(), AppErr> {
    *expected_line = line!() + 1;
    plain_result()?;

    Ok(())
}

pub fn reject_if_missing(missing: bool, expected_line: &mut u32) -> ErResult<(), AppErr> {
    if missing {
        *expected_line = line!() + 1;
        er_bail!(AppErr);
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

#[test]
pub fn bail() {
    let mut expected_line = 0;
    let result = (|| -> ErResult<(), AppErr> {
        expected_line = line!() + 1;
        er_bail!(AppErr,);
    })();
    let tree = result.unwrap_err();
    assert!(tree.nodes.is_empty());
    #[cfg(feature = "src_locations")]
    assert_eq!(tree.src_location.line(), expected_line);

    let result = (|| -> Result<(), ErReport<AppErr>> {
        expected_line = line!() + 1;
        er_bail!(AppErr);
    })();
    let report = result.unwrap_err();
    assert_eq!(report.tree.top.to_string(), "AppErr");
    #[cfg(feature = "src_locations")]
    assert_eq!(report.tree.src_location.line(), expected_line);

    let result = (|| -> Result<(), ErTop<AppErr>> {
        expected_line = line!() + 1;
        er_bail!(AppErr);
    })();
    let top = result.unwrap_err();
    assert_eq!(top.to_string(), "AppErr");
    #[cfg(feature = "src_locations")]
    assert_eq!(top.tree.src_location.line(), expected_line);

    let tree = ErTree::new(AppErr, [ChildErr(7)]);
    #[cfg(feature = "stack_traces")]
    let tree = tree.er_trace();
    let nodes = tree.nodes.as_ptr();
    #[cfg(feature = "stack_traces")]
    let traces = tree.stack_traces.as_ptr();
    let result = (|| -> ErResult<(), AppErr> { er_bail!(tree.into_er_report()) })();
    let tree = result.unwrap_err();
    assert_eq!(tree.nodes.as_ptr(), nodes);
    assert_eq!(tree.er_find::<ChildErr>().unwrap().0, 7);
    #[cfg(feature = "stack_traces")]
    assert_eq!(tree.stack_traces.as_ptr(), traces);
}

#[derive(Er)]
pub struct ChildErr(pub u8);
#[test]
pub fn er_val() -> Result<(), ErReport<ChildErr>> {
    let calls = Cell::new(0);
    let input: Result<u8, u8> = Ok(7);
    let result: ErResult<u8, ChildErr> = input.er_val(|value| {
        calls.set(calls.get() + 1);
        ChildErr(value)
    });

    assert_eq!(result.er_report()?, 7);
    assert_eq!(calls.get(), 0);

    let result: Result<(), u8> = Err(7);
    let _expected_line = line!() + 1;
    let error = result.er_val(ChildErr).unwrap_err();

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

#[derive(Er)]
pub struct EmptyErr;

#[derive(Er)]
pub struct FieldErr(pub String);

#[derive(Default, Er)]
pub struct FactoryErr;
impl From<FactoryErr> for String {
    fn from(error: FactoryErr) -> Self {
        error.to_string()
    }
}

#[derive(Er)]
pub enum PresetErr {
    Invalid(u8),
}

#[derive(Er)]
pub struct BoundaryErr;

fn empty() -> ErResult<(), EmptyErr> {
    Err::<(), _>(std::fmt::Error).er(())
}

fn fields() -> ErResult<(), FieldErr> {
    Err::<(), _>(std::fmt::Error).er(|_| "field")
}

fn boundary<T, E>(result: Result<T, E>) -> ErResult<T, BoundaryErr>
where
    E: IntoErPart,
{
    result.er(())
}

#[test]
pub fn context_spellings() {
    assert!(empty().is_err());
    assert_eq!(fields().unwrap_err().top.0, "field");

    let made = Err::<(), _>(std::fmt::Error).er(FactoryErr::new);
    assert_eq!(
        Any::type_id(&made.as_ref().unwrap_err().top),
        TypeId::of::<FactoryErr>()
    );

    let defaulted = Err::<(), _>(std::fmt::Error).er(FactoryErr::default);
    assert_eq!(
        Any::type_id(&defaulted.as_ref().unwrap_err().top),
        TypeId::of::<FactoryErr>()
    );

    let input = 7;
    let preset = Err::<(), _>(std::fmt::Error).er(|| PresetErr::invalid(input));
    assert_eq!(
        Any::type_id(&preset.as_ref().unwrap_err().top),
        TypeId::of::<PresetErr>()
    );

    let factory = FactoryErr::new;
    assert!(Err::<(), _>(std::fmt::Error).er(factory).is_err());
    assert!(Err::<(), _>(std::fmt::Error).er(factory).is_err());

    assert!(boundary(Err::<(), _>(std::fmt::Error).er(FactoryErr::new)).is_err());
}

#[test]
pub fn local_roots() {
    let root = ErTree::from(local_error());
    assert_eq!(root.top.attempts.get(), 7);
    assert!(root.er_report().to_string().contains("local"));
    assert!(root.er_find::<LocalErr>().is_some());

    let converted = ErTree::from(local_error());
    assert!(converted.nodes.is_empty());

    let missing: Option<()> = None;
    assert!(
        missing
            .er::<LocalErr>(local_error)
            .unwrap_err()
            .nodes
            .is_empty()
    );

    let status: Result<(), u8> = Err(7);
    let mapped = status.er_val(|attempts| LocalErr::new(Cell::new(attempts), "mapped"));

    assert_eq!(mapped.unwrap_err().top.attempts.get(), 7);

    let result: Result<(), ChildErr> = Err(ChildErr(1));
    let context = result.er::<LocalErr>(local_error).unwrap_err();

    assert_eq!(context.er_find::<ChildErr>().unwrap().0, 1);

    let context = ChildErr(2).er::<LocalErr>(local_error);
    assert_eq!(context.er_find::<ChildErr>().unwrap().0, 2);

    let grouped = ErTree::new(local_error(), [ChildErr(3)]);
    assert_eq!(grouped.er_find::<ChildErr>().unwrap().0, 3);

    let result: Result<(), ChildErr> = Err(ChildErr(5));
    let collected: ErTree<LocalErr> = er_all!(local_error, [result]).unwrap_err();

    assert_eq!(collected.er_find::<ChildErr>().unwrap().0, 5);
}

#[derive(Er)]
pub struct ParentErr(pub u8);
impl From<u8> for ParentErr {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

#[test]
pub fn raw_context() {
    let _line = line!() + 1;
    let error = ChildErr(1).er::<AppErr>(());
    assert_eq!(error.er_find::<ChildErr>().unwrap().0, 1);
    #[cfg(feature = "src_locations")]
    {
        assert_eq!(error.src_location.line(), _line);
        assert_eq!(error.nodes[0].src_location.line(), _line);
    }

    let error: ErTree<ParentErr> = ChildErr(2).er(|_| 7u8);
    assert_eq!(error.top.0, 7);
    assert_eq!(error.er_find::<ChildErr>().unwrap().0, 2);

    let error = ChildErr(3).er::<ParentErr>(|_| 8u8);
    assert_eq!(error.top.0, 8);

    let error = ChildErr(5).er::<LocalErr>(|_| (Cell::new(6), "raw"));
    assert_eq!(error.top.attempts.get(), 6);
    assert_eq!(&*error.top.label, "raw");
    assert_eq!(error.er_find::<ChildErr>().unwrap().0, 5);

    let error = ChildErr(4).er(|| ParentErr::new(9));
    assert_eq!(error.top.0, 9);
    assert_eq!(error.er_find::<ChildErr>().unwrap().0, 4);
}

#[test]
pub fn er_with() {
    let calls = Cell::new(0);
    let ok: Result<u8, ChildErr> = Ok(7);
    let result: ErResult<u8, ParentErr> = ok.er_with(|e| {
        calls.set(calls.get() + 1);
        ParentErr(e.0)
    });
    assert!(matches!(result, Ok(7)));
    assert_eq!(calls.get(), 0);

    let result: Result<(), ChildErr> = Err(ChildErr(1));
    let error = result.er_with::<ParentErr>(|e| e.0.into()).unwrap_err();
    assert_eq!(error.top.0, 1);
    assert_eq!(error.er_find::<ChildErr>().unwrap().0, 1);

    let error = ChildErr(2).er_with::<ParentErr>(|e| e.0.into());
    assert_eq!(error.top.0, 2);
    assert_eq!(error.er_find::<ChildErr>().unwrap().0, 2);

    let error = ErTree::from(ChildErr(3)).er_with::<ParentErr>(|t| t.top.0.into());
    assert_eq!(error.top.0, 3);
    assert_eq!(error.er_find::<ChildErr>().unwrap().0, 3);
}
