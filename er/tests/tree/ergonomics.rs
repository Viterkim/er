use er::*;
use std::cell::Cell;

#[derive(Er)]
pub struct AppErr;
pub fn plain_result() -> Result<(), AppErr> {
    Err(AppErr)
}

pub fn converted() -> ErResult<(), AppErr> {
    plain_result()?;

    Ok(())
}

#[test]
pub fn question_mark() {
    assert!(converted().unwrap_err().nodes.is_empty());
}

#[test]
pub fn bail() {
    let _ = (|| -> ErResult<(), AppErr> { er_bail!(AppErr,) })();
    let _ = (|| -> ErResult<(), AppErr> { er_bail!(()) })();
    let _ = (|| -> Result<(), ErReport<AppErr>> { er_bail!(AppErr) })();
    let _ = (|| -> Result<(), ErTop<AppErr>> { er_bail!(AppErr) })();

    let tree = ErTree::new(AppErr, [ChildErr(7)]);
    let result = (|| -> ErResult<(), AppErr> { er_bail!(tree.into_er_report()) })();
    assert_eq!(result.unwrap_err().er_find::<ChildErr>().unwrap().0, 7);
}

#[derive(Er)]
pub struct ChildErr(pub u8);
#[test]
pub fn er_val() -> Result<(), ErReport<ChildErr>> {
    fn convert(input: Result<u8, u8>, calls: &Cell<usize>) -> ErResult<u8, ChildErr> {
        input.er_val(|v| {
            calls.set(calls.get() + 1);
            v
        })
    }

    let calls = Cell::new(0);
    convert(Ok(7), &calls)?;
    assert_eq!(calls.get(), 0);
    assert_eq!(convert(Err(7), &calls).unwrap_err().top.0, 7);

    let error = Err::<(), _>(7u8)
        .er_val::<PresetErr, _>(PresetErr::invalid)
        .unwrap_err();
    let PresetErr::Invalid(_) = error.top;

    let error = Err::<(), _>(String::from("foreign"))
        .er_val::<std::io::Error, _>(|v| er_built(std::io::Error::other(v)))
        .unwrap_err();
    assert_eq!(error.top.to_string(), "foreign");

    let tree = ErTree::new(ChildErr(8), [std::fmt::Error]);
    let error = Err::<(), _>(tree)
        .er_val::<ChildErr, _>(|tree| tree.top.0)
        .unwrap_err();
    assert!(error.nodes.is_empty());

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

fn boundary<T, E>(result: Result<T, E>) -> ErResult<T, BoundaryErr>
where
    E: IntoErPart,
{
    result.er(())
}

#[test]
pub fn context_spellings() {
    let _ = Err::<(), _>(std::fmt::Error).er(FactoryErr::new);
    let _ = Err::<(), _>(std::fmt::Error).er(FactoryErr::default);

    let input = 7;
    let _ = Err::<(), _>(std::fmt::Error).er(|| PresetErr::invalid(input));

    assert!(boundary(Err::<(), _>(std::fmt::Error).er(FactoryErr::new)).is_err());
}

#[test]
pub fn local_roots() {
    let root = ErTree::from(local_error());
    assert!(root.er_report().to_string().contains("local"));

    let missing: Option<()> = None;
    let _ = missing.er::<LocalErr>(local_error);

    let status: Result<(), u8> = Err(7);
    let _: ErResult<(), LocalErr> = status.er_val(|v| (Cell::new(v), "mapped"));

    let result: Result<(), ChildErr> = Err(ChildErr(1));
    let _ = result.er::<LocalErr>(local_error);
    let _ = ChildErr(2).er::<LocalErr>(local_error);
    let _ = ErTree::new(local_error(), [ChildErr(3)]);

    let result: Result<(), ChildErr> = Err(ChildErr(5));
    let _: ErResult<(), LocalErr> = er_all!(local_error, [result]);
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
    let error = ChildErr(1).er(AppErr::new);
    assert_eq!(error.er_find::<ChildErr>().unwrap().0, 1);

    let error: ErTree<ParentErr> = ChildErr(2).er(|_| 7u8);
    assert_eq!(error.top.0, 7);

    let _ = ChildErr(3).er::<ParentErr>(|_| 8u8);
    let _ = ChildErr(5).er::<LocalErr>(|_| (Cell::new(6), "raw"));
    let _ = ChildErr(4).er(|| ParentErr::new(9));
}

#[test]
pub fn er_with() {
    fn convert<T>(input: Result<T, ChildErr>, calls: &Cell<usize>) -> ErResult<T, ParentErr> {
        let value = input.er_with(|err| {
            calls.set(calls.get() + 1);
            err.0
        })?;
        Ok(value)
    }

    let calls = Cell::new(0);
    let _ = convert(Ok::<u8, _>(7), &calls);
    assert_eq!(calls.get(), 0);

    let error = convert(Err::<(), _>(ChildErr(1)), &calls).unwrap_err();
    assert_eq!(error.top.0, 1);
    assert_eq!(error.er_find::<ChildErr>().unwrap().0, 1);

    let _ = ChildErr(2).er_with::<ParentErr, _>(|err| err.0);
    let error = ErTree::from(ChildErr(3)).er_with::<ParentErr, _>(|err| err.0);
    let error = Err::<(), _>(error)
        .er_with::<ParentErr, _>(|err| err.0 + 1)
        .unwrap_err();
    assert_eq!(error.er_find_all::<ParentErr>().count(), 2);

    let _: ErTree<ParentErr> = error.er_with(|err| er_built(ParentErr(err.0 + 1)));
}

#[test]
pub fn er_with_tree() {
    let calls = Cell::new(0);
    let ok: ErResult<u8, ParentErr> = Ok(7);
    let _: ErResult<u8, ParentErr> = ok.er_with_tree(|tree| {
        calls.set(calls.get() + 1);
        tree.top.0
    });
    assert_eq!(calls.get(), 0);

    let tree = ChildErr(3).er::<ParentErr>(|_| 7u8);
    let tree: ErTree<ParentErr> = tree.er_with_tree(|tree| {
        er_built(ParentErr(
            tree.top.0 + tree.er_find::<ChildErr>().unwrap().0,
        ))
    });
    assert_eq!(tree.top.0, 10);

    let tree = Err::<(), _>(tree)
        .er_with_tree::<ParentErr, _>(|tree| tree.er_find::<ChildErr>().unwrap().0)
        .unwrap_err();
    assert_eq!(tree.er_find_all::<ParentErr>().count(), 3);
}
