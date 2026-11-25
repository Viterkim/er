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
    let _ = (|| -> ErResult<(), AppErr> { er_bail!() })();
    let _ = (|| -> ErTest { er_bail!(AppErr) })();

    let tree = Box::new(ErTree::new(AppErr, [SubErr(7)]));
    let result = (|| -> ErResult<(), AppErr> { er_bail!(tree) })();
    assert_eq!(result.unwrap_err().er_find::<SubErr>().unwrap().0, 7);
}

#[test]
pub fn bail_with_sub_errors() {
    let bail = || {
        let context = |_| ();
        er_bail!(context, [SubErr(7), std::fmt::Error]);
    };
    let result: ErResult<(), AppErr> = bail();
    assert_eq!(result.unwrap_err().er_find::<SubErr>().unwrap().0, 7);

    let failures = vec![SubErr(85)];
    let result = (|| -> ErTest { er_bail!("import failed", failures) })();
    let failure = result.unwrap_err();
    assert_eq!(failure.nodes[0].error.to_string(), "import failed");
    assert!(failure.nodes[0].nodes[0].error.is::<SubErr>());

    let result = (|| -> ErMain { er_bail!((), [SubErr(7)]) })();
    assert!(result.unwrap_err().tree.nodes[0].error.is::<SubErr>());
}

#[test]
pub fn borrowed_factory_fields() {
    #[derive(Er)]
    struct NameErr(String);

    let mut name = String::from("kat");
    let error: ErTree<NameErr> = er_new!(|| {
        name.push('t');
        name.as_str()
    });
    assert_eq!(error.top.0, "katt");

    let result = (|| -> ErResult<(), NameErr> {
        er_bail!(
            || {
                name.push('e');
                name.as_str()
            },
            [SubErr(7)]
        );
    })();
    assert_eq!(result.unwrap_err().top.0, "katte");

    let result: ErResult<_, NameErr> = er_try!(
        |_| {
            name.push('n');
            name.as_str()
        },
        [SubErr(7)]
    );
    assert_eq!(result.unwrap_err().top.0, "katten");
}

#[test]
pub fn bail_with_shared_top() {
    let shared = ErTree::new(AppErr, [SubErr(7)]).er_share();
    let part = (&shared).into_er_part();
    assert!(part.er_error().is::<AppErr>());

    let failure = (|| -> ErTest { er_bail!(part, [std::fmt::Error]) })().unwrap_err();
    assert!(failure.er_at_path(&[0, 0]).unwrap().is::<SubErr>());
    assert!(failure.er_at_path(&[1]).unwrap().is::<std::fmt::Error>());
}

#[derive(Er)]
pub struct SubErr(pub u8);
#[test]
pub fn er_val() -> ErTest {
    fn convert(input: Result<u8, u8>, calls: &Cell<usize>) -> ErResult<u8, SubErr> {
        input.er_val(|v| {
            calls.set(calls.get() + 1);
            v
        })
    }

    let calls = Cell::new(0);
    convert(Ok(7), &calls)?;
    assert_eq!(calls.get(), 0);
    assert_eq!(convert(Err(7), &calls).unwrap_err().top.0, 7);

    let first = Err::<(), _>(7u8).er_val(|n| PresetErr::invalid(n).er_save("port", "busy"));
    let second = Err::<(), _>(SubErr(8)).er_with(|err| PresetErr::invalid(err.0));
    let error = er_add!(er_new!(AppErr::new), [first, second]);
    assert_eq!(error.er_find_all::<PresetErr>().count(), 2);

    let error = Err::<(), _>(String::from("foreign"))
        .er_val::<std::io::Error, _>(std::io::Error::other)
        .unwrap_err();
    assert_eq!(error.top.to_string(), "foreign");

    let tree = ErTree::new(SubErr(8), [std::fmt::Error]);
    let error = Err::<(), _>(tree)
        .er_val::<SubErr, _>(|tree| tree.top.0)
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
    let root = er_new!(local_error());
    assert!(root.er_report().to_string().contains("local"));

    let missing: Option<()> = None;
    let _ = missing.er::<LocalErr>(local_error);

    let status: Result<(), u8> = Err(7);
    let _: ErResult<(), LocalErr> = status.er_val(|v| (Cell::new(v), "mapped"));

    let result: Result<(), SubErr> = Err(SubErr(1));
    let _ = result.er::<LocalErr>(local_error);
    let _ = SubErr(2).er::<LocalErr>(local_error);
    let _ = ErTree::new(local_error(), [SubErr(3)]);

    let result: Result<(), SubErr> = Err(SubErr(5));
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
    let error = SubErr(1).er(AppErr::new);
    assert_eq!(error.er_find::<SubErr>().unwrap().0, 1);

    let error: ErTree<ParentErr> = SubErr(2).er(|_| 7u8);
    assert_eq!(error.top.0, 7);

    let _ = SubErr(3).er::<ParentErr>(|_| 8u8);
    let _ = SubErr(5).er::<LocalErr>(|_| (Cell::new(6), "raw"));
    let _ = SubErr(4).er(|| ParentErr::new(9));
}

#[test]
pub fn er_with() {
    fn convert<T>(input: Result<T, SubErr>, calls: &Cell<usize>) -> ErResult<T, ParentErr> {
        let value = input.er_with(|err| {
            calls.set(calls.get() + 1);
            err.0
        })?;
        Ok(value)
    }

    let calls = Cell::new(0);
    let _ = convert(Ok::<u8, _>(7), &calls);
    assert_eq!(calls.get(), 0);

    let error = convert(Err::<(), _>(SubErr(1)), &calls).unwrap_err();
    assert_eq!(error.top.0, 1);
    assert_eq!(error.er_find::<SubErr>().unwrap().0, 1);

    let _ = SubErr(2).er_with::<ParentErr, _>(|err| err.0);
    let error = ErTree::from(SubErr(3)).er_with::<ParentErr, _>(|err| err.0);
    let error = Err::<(), _>(error)
        .er_with::<ParentErr, _>(|err| err.0 + 1)
        .unwrap_err();
    assert_eq!(error.er_find_all::<ParentErr>().count(), 2);

    let _: ErTree<ParentErr> = error.er_with(|err| ParentErr(err.0 + 1));

    let pair = (SubErr(4), String::from("device did not answer"));
    let error: ErTree<ParentErr> = pair.er_save("retry", "same result").er_with(|err| err.0);
    assert_eq!(error.top.0, 4);
    let report = error.er_report();
    assert!(report.contains("device did not answer"));
    assert!(report.contains("same result"));
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

    let tree = SubErr(3).er::<ParentErr>(|_| 7u8);
    let tree: ErTree<ParentErr> =
        tree.er_with_tree(|tree| ParentErr(tree.top.0 + tree.er_find::<SubErr>().unwrap().0));
    assert_eq!(tree.top.0, 10);

    let tree = Err::<(), _>(tree)
        .er_with_tree::<ParentErr, _>(|tree| tree.er_find::<SubErr>().unwrap().0)
        .unwrap_err();
    assert_eq!(tree.er_find_all::<ParentErr>().count(), 3);
}

#[test]
pub fn er_main() {
    #[derive(Er)]
    enum MainCaseErr {
        Missing,
    }

    let pair = (|| -> ErMain {
        let result: Result<(), (SubErr, String)> = Err((SubErr(85), String::from("saved detail")));
        result.er(())?;
        Ok(())
    })()
    .unwrap_err();
    let printed = format!("{pair:?}");
    assert!(printed.contains("MainError"));
    assert!(printed.contains("SubErr(85)"));
    assert!(printed.contains("saved detail"));

    let missing = (|| -> ErMain {
        None::<u8>.er(())?;
        Ok(())
    })()
    .unwrap_err();
    assert!(format!("{missing:?}").contains("MainError"));

    let all = (|| -> ErMain {
        er_all!((), [Err::<(), _>(SubErr(1)), Err::<(), _>(SubErr(2))])?;
        Ok(())
    })()
    .unwrap_err();
    assert_eq!(all.tree.er_find_all::<SubErr>().count(), 2);

    let bailed = (|| -> ErMain { er_bail!(ErMainError) })().unwrap_err();
    assert!(format!("{bailed:?}").contains("MainError"));

    let unit_bail = (|| -> ErMain { er_bail!(()) })().unwrap_err();
    assert!(format!("{unit_bail:?}").contains("MainError"));

    let custom = (|| -> ErMain<MainCaseErr> { er_bail!(MainCaseErr::Missing) })().unwrap_err();
    assert!(matches!(custom.tree.top, MainCaseErr::Missing));
    assert!(format!("{custom:?}").contains("Missing"));

    let custom_tree = (|| -> ErMain<MainCaseErr> {
        Err::<(), _>(SubErr(9)).er(|| MainCaseErr::Missing)?;
        Ok(())
    })()
    .unwrap_err();
    assert!(custom_tree.tree.er_contains::<SubErr>());
}
