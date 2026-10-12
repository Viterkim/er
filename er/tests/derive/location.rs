use er::*;
use std::{cell::Cell, fmt, panic::Location};

#[derive(Er)]
#[er(format = "invalid {value:?} @ {location}")]
struct Located {
    value: String,
    #[er(src_location)]
    location: ErSrcLocation,
}

#[test]
fn constructors() {
    let expected = line!() + 1;
    let err = Located::new("banana");
    assert_eq!(err.location.line(), expected);
    assert_eq!(err.value, "banana");
    assert!(err.to_string().contains(file!()));

    let expected = line!() + 1;
    let err = Located::from(("banana",));
    assert_eq!(err.location.line(), expected);

    let creation_line = line!() + 1;
    let make = || Located::new("banana");
    let err = make();
    assert_eq!(err.location.line(), creation_line);

    #[derive(Er)]
    enum Variant {
        Failed {
            value: u8,
            #[er(src_location)]
            location: ErSrcLocation,
        },
    }

    let expected = line!() + 1;
    let variant = Variant::failed(85);
    let Variant::Failed { value, location } = variant;
    assert_eq!(value, 85);
    assert_eq!(location.line(), expected);

    #[derive(Er)]
    struct Empty {
        #[er(src_location)]
        location: &'static Location<'static>,
    }

    let expected = line!() + 1;
    let err = Empty::new();
    assert_eq!(err.location.line(), expected);

    let expected = line!() + 1;
    let err = Empty::from(());
    assert_eq!(err.location.line(), expected);
}

#[test]
fn helpers() {
    fn context(line: &Cell<u32>) -> ErResult<(), Located> {
        line.set(line!() + 1);
        Err(fmt::Error).er(|_| "context")
    }

    fn value(line: &Cell<u32>) -> ErResult<(), Located> {
        let fields = |v: &'static str| v;
        line.set(line!() + 1);
        Err("value").er_val(fields)
    }

    fn borrowed(line: &Cell<u32>) -> ErResult<(), Located> {
        line.set(line!() + 1);
        Err(fmt::Error).er_with(|err| err.to_string())
    }

    fn borrowed_tree(line: &Cell<u32>) -> ErResult<(), Located> {
        let input: ErResult<(), Located> = Err(ErTree::from(Located::new("inner")));
        line.set(line!() + 1);
        input.er_with_tree(|tree| tree.top.value.clone())
    }

    fn all(line: &Cell<u32>) -> ErResult<(), Located> {
        line.set(line!() + 1);
        er_all!(|_| "all", [Err::<(), _>(fmt::Error)])
    }

    fn iterator(line: &Cell<u32>) -> ErResult<(), Located> {
        line.set(line!() + 1);
        er_all!(|_| "iterator", [Err::<(), _>(fmt::Error)].into_iter())
    }

    fn collect(line: &Cell<u32>) -> ErResult<(), Located> {
        let input = [Err::<(), _>(fmt::Error)].into_iter();
        line.set(line!() + 1);
        input.er_collect::<Vec<_>, _>(|_| "collect").map(|_| ())
    }

    fn collect_all(line: &Cell<u32>) -> ErResult<(), Located> {
        let input = [Err::<(), _>(fmt::Error)].into_iter();
        line.set(line!() + 1);
        input.er_collect_all::<Vec<_>, _>(|_| "all").map(|_| ())
    }

    fn values(line: &Cell<u32>) -> ErResult<(), Located> {
        line.set(line!() + 1);
        er_try!(|_| "try", [Err::<(), _>(fmt::Error)])?;
        Ok(())
    }

    fn bail(line: &Cell<u32>) -> ErResult<(), Located> {
        line.set(line!() + 1);
        er_bail!(|_| "bail");
    }

    let line = Cell::new(0);
    for make in [
        context,
        value,
        borrowed,
        borrowed_tree,
        all,
        iterator,
        collect,
        collect_all,
        values,
        bail,
    ] {
        let err = make(&line).unwrap_err();
        assert_eq!(err.top.location.line(), line.get());
        assert_eq!(err.top.location.file(), file!());
    }

    #[derive(Er)]
    struct Defaulted {
        value: String,
        #[er(src_location)]
        location: ErSrcLocation,
        #[er(default)]
        retries: u32,
    }

    let expected = line!() + 1;
    let defaulted: ErResult<(), Defaulted> = Err("default").er_val(|v| v);
    let defaulted = defaulted.unwrap_err().top;
    assert_eq!(defaulted.location.line(), expected);
    assert_eq!(defaulted.retries, 0);
    assert_eq!(defaulted.value, "default");

    let expected = line!() + 1;
    let explicit: ErResult<(), Defaulted> = Err("explicit").er_val(|v| (v, 85));
    let explicit = explicit.unwrap_err().top;
    assert_eq!(explicit.location.line(), expected);
    assert_eq!(explicit.retries, 85);

    let calls = Cell::new(0);
    let successful: ErResult<(), Located> = Ok::<_, &str>(()).er_val(|v| {
        calls.set(calls.get() + 1);
        v
    });
    assert!(successful.is_ok());
    assert_eq!(calls.get(), 0);

    #[derive(Er)]
    #[er(format = "saved")]
    struct Saved {
        #[er(into_report_string)]
        diagnostics: String,
        #[er(src_location)]
        location: ErSrcLocation,
        #[er(default)]
        data: (),
    }

    fn boundary(line: &Cell<u32>) -> Result<(), Saved> {
        let input = context(line);
        line.set(line!() + 1);
        input.er_into(|report| report)
    }

    let input = context(&line);
    let expected = line!() + 1;
    let saved: Saved = input.unwrap_err().er_into(|report| report);
    assert_eq!(saved.location.line(), expected);
    assert!(saved.diagnostics.contains("context"));
    let () = saved.data;
    let saved = boundary(&line).unwrap_err();
    assert_eq!(saved.location.line(), line.get());
}
