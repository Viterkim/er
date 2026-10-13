use er::*;
use std::{cell::Cell, fmt};

thread_local! {
    static CALLS: Cell<usize> = const { Cell::new(0) };
}

fn label() -> String {
    CALLS.with(|calls| calls.set(calls.get() + 1));
    "fallback".into()
}

#[derive(Er)]
#[er(format = "{path}")]
struct Context<T> {
    path: String,
    #[er(default)]
    data: T,
    #[er(default = label())]
    label: String,
}

struct Payload;

#[test]
fn macros() {
    fn all(input: Result<u8, fmt::Error>) -> ErResult<(), Context<()>> {
        er_all!(|_| "batch", [input])
    }

    fn bail() -> ErResult<(), Context<()>> {
        er_bail!(|_| "settings");
    }

    let iterator: ErResult<(), Context<()>> =
        er_all!(|_| "batch", [Err::<u8, _>(fmt::Error)].into_iter());
    let values: ErResult<(u8,), Context<()>> = er_try!(|_| "batch", [Err::<u8, _>(fmt::Error)]);
    let errors = [
        all(Err(fmt::Error)).unwrap_err(),
        iterator.unwrap_err(),
        values.unwrap_err(),
        bail().unwrap_err(),
    ];
    assert!(errors.into_iter().all(|tree| tree.top.label == "fallback"));
}

#[test]
fn fields() {
    fn value(input: Result<(), &str>) -> ErResult<(), Context<()>> {
        input.er_val(|v| v)
    }

    CALLS.with(|calls| calls.set(0));
    let _ = value(Ok(()));
    assert_eq!(CALLS.with(Cell::get), 0);

    assert_eq!(value(Err("settings")).unwrap_err().top.label, "fallback");

    fn explicit(input: Result<(), &str>) -> ErResult<(), Context<Payload>> {
        input.er_val(|v| (v, Payload, "supplied"))
    }

    let tree = explicit(Err("settings")).unwrap_err();
    let Payload = tree.top.data;
    assert_eq!(tree.top.label, "supplied");

    #[derive(Er)]
    struct AllDefault {
        #[er(default)]
        count: u8,
        #[er(default = label())]
        label: String,
    }

    let empty: [ErResult<(), AllDefault>; 4] = [
        Err(fmt::Error).er(()),
        Err(7_u8).er_val(|_| ()),
        Err(fmt::Error).er_with(|_| ()),
        (|| er_bail!(|_| ()))(),
    ];
    assert_eq!(
        empty.map(|result| result.unwrap_err().top.label),
        ["fallback"; 4]
    );
    let _: Result<(), AllDefault> = Err::<(), _>(ErTree::from(fmt::Error)).er_into(|_| ());

    type Pair = ((u8, u8),);

    #[derive(Er)]
    struct Alias {
        pair: Pair,
        #[er(default)]
        data: u8,
    }

    fn alias(pair: Pair) -> ErResult<(), Alias> {
        Err(pair).er_val(|v| v)
    }

    assert_eq!(alias(((7, 85),)).unwrap_err().top.pair, ((7, 85),));

    fn callback() -> u8 {
        85
    }

    #[derive(ErFormat)]
    #[er(format = "{name}")]
    struct Data {
        name: String,
        #[er(default = callback)]
        callback: fn() -> u8,
    }

    fn data() -> Result<(), Data> {
        value(Err("callback")).er_into(|report| report.er_top_string())
    }

    assert_eq!((data().unwrap_err().callback)(), 85);

    #[derive(Er)]
    struct Optional {
        path: String,
        #[er(default = Some(85))]
        value: Option<u8>,
    }

    fn supplied_none() -> ErResult<(), Optional> {
        Err("settings").er_val(|v| (v, None))
    }

    assert!(supplied_none().unwrap_err().top.value.is_none());
}

#[test]
fn unit_field() {
    #[derive(Er)]
    struct Optional {
        #[er(default)]
        value: Option<()>,
    }

    fn omitted() -> ErResult<(), Optional> {
        Err(7_u8).er_val::<Optional, ErDefaults>(|_| ())
    }

    fn supplied() -> ErResult<(), Optional> {
        Err(7_u8).er_val::<Optional, ErFields>(|_| ())
    }

    assert!(omitted().unwrap_err().top.value.is_none());
    assert_eq!(supplied().unwrap_err().top.value, Some(()));

    let empty: ErResult<(), Optional> = Err(fmt::Error).er(());
    assert!(empty.unwrap_err().top.value.is_none());
}

#[derive(Er)]
#[er(format = "{diagnostics}")]
struct Saved<T> {
    #[er(into_report_string)]
    diagnostics: String,
    #[er(default)]
    data: T,
}

#[derive(Er)]
#[er(format = "{diagnostics}")]
struct Request<T> {
    #[er(into_report_string)]
    diagnostics: String,
    request: String,
    #[er(default)]
    data: T,
}

#[test]
fn reports() {
    fn input() -> ErResult<(), Context<()>> {
        Err(fmt::Error).er(|_| "settings")
    }

    fn saved() -> Result<(), Saved<()>> {
        input().er_into(|report| report)
    }

    fn supplied() -> Result<(), Saved<Payload>> {
        input().er_into(|report| (report, Payload))
    }

    fn request() -> Result<(), Request<()>> {
        input().er_into(|report| (report, "request"))
    }

    let err = saved().unwrap_err();
    assert!(err.diagnostics.contains("settings"));
    let () = err.data;
    let Payload = supplied().unwrap_err().data;

    let err = request().unwrap_err();
    assert_eq!(err.request, "request");
    let () = err.data;
}
