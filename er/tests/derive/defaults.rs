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
fn aggregation() {
    fn all(input: Result<u8, fmt::Error>, line: &Cell<u32>) -> ErResult<(), Context<()>> {
        line.set(line!() + 1);
        er_all!(|_| "batch", [input])
    }

    fn iterator(input: Result<u8, fmt::Error>, line: &Cell<u32>) -> ErResult<(), Context<()>> {
        line.set(line!() + 1);
        er_all!(|_| "batch", [input].into_iter())
    }

    fn values<'a>(
        input: Result<u8, fmt::Error>,
        text: &'a str,
        line: &Cell<u32>,
    ) -> ErResult<(u8, &'a str), Context<()>> {
        line.set(line!() + 1);
        er_try!(|_| "batch", [input, Ok::<_, fmt::Error>(text)])
    }

    CALLS.with(|calls| calls.set(0));
    let line = Cell::new(0);
    let text = String::from("borrowed");
    assert!(all(Ok(7), &line).is_ok());
    assert!(iterator(Ok(7), &line).is_ok());
    let (number, borrowed) = values(Ok(7), &text, &line).unwrap_report();
    assert_eq!(number, 7);
    assert!(std::ptr::eq(borrowed, text.as_str()));
    assert_eq!(CALLS.with(Cell::get), 0);

    for aggregate in [all, iterator] {
        let result = aggregate(Err(fmt::Error), &line);
        let tree = result.unwrap_err();
        assert_eq!(tree.top.path, "batch");
        assert_eq!(tree.top.label, "fallback");
        assert_eq!(tree.nodes.len(), 1);
        assert!(tree.er_contains::<fmt::Error>());

        #[cfg(feature = "src_locations")]
        assert_eq!(tree.src_location.line(), line.get());

        let tree = er_add!(tree, [Err::<(), _>(fmt::Error)]);
        assert_eq!(tree.nodes.len(), 2);
        assert_eq!(tree.top.label, "fallback");
    }
    assert_eq!(CALLS.with(Cell::get), 2);

    let tree = values(Err(fmt::Error), &text, &line).unwrap_err();
    assert_eq!(tree.top.label, "fallback");
    assert!(tree.er_contains::<fmt::Error>());
    #[cfg(feature = "src_locations")]
    assert_eq!(tree.src_location.line(), line.get());

    fn full() -> ErResult<(), Context<Payload>> {
        er_all!(
            |_| ("batch", Payload, "supplied"),
            [Err::<(), _>(fmt::Error)]
        )
    }

    fn full_values() -> ErResult<((),), Context<Payload>> {
        er_try!(
            |_| ("batch", Payload, "supplied"),
            [Err::<(), _>(fmt::Error)]
        )
    }

    for tree in [full().unwrap_err(), full_values().unwrap_err()] {
        let Payload = tree.top.data;
        assert_eq!(tree.top.label, "supplied");
    }
    assert_eq!(CALLS.with(Cell::get), 3);

    #[derive(Er)]
    struct Empty {
        #[er(default)]
        attempts: u32,
    }

    fn empty_all(input: Result<(), fmt::Error>) -> ErResult<(), Empty> {
        er_all!(|_| (), [input])
    }

    fn empty_iterator(input: Result<(), fmt::Error>) -> ErResult<(), Empty> {
        er_all!(|_| (), [input].into_iter())
    }

    fn empty_values(input: Result<(), fmt::Error>) -> ErResult<((),), Empty> {
        er_try!(|_| (), [input])
    }

    fn empty_bail() -> ErResult<(), Empty> {
        er_bail!(|_| ());
    }

    for tree in [
        empty_all(Err(fmt::Error)).unwrap_err(),
        empty_iterator(Err(fmt::Error)).unwrap_err(),
        empty_values(Err(fmt::Error)).unwrap_err(),
        empty_bail().unwrap_err(),
    ] {
        assert_eq!(tree.top.attempts, 0);
    }
}

#[test]
fn fields() {
    fn value(input: Result<(), &str>) -> ErResult<(), Context<()>> {
        input.er_val(|v| v)
    }

    fn context(input: Result<(), fmt::Error>) -> ErResult<(), Context<()>> {
        input.er(|_| "read")
    }

    fn borrowed(input: Result<(), fmt::Error>) -> ErResult<(), Context<()>> {
        input.er_with(|err| err.to_string())
    }

    fn boundary(input: ErResult<(), Context<()>>) -> Result<(), Context<()>> {
        input.er_into(|report| report.er_report_string())
    }

    CALLS.with(|calls| calls.set(0));
    assert!(value(Ok(())).is_ok());
    assert!(context(Ok(())).is_ok());
    assert!(borrowed(Ok(())).is_ok());
    assert!(boundary(Ok(())).is_ok());
    assert_eq!(CALLS.with(Cell::get), 0);

    let tree = value(Err("settings")).unwrap_err();
    assert_eq!(tree.top.path, "settings");
    assert_eq!(tree.top.label, "fallback");
    assert!(tree.nodes.is_empty());
    assert!(
        context(Err(fmt::Error))
            .unwrap_err()
            .er_contains::<fmt::Error>()
    );
    assert!(
        borrowed(Err(fmt::Error))
            .unwrap_err()
            .er_contains::<fmt::Error>()
    );
    let err = boundary(Err(tree)).unwrap_err();
    assert!(err.path.contains("settings"));
    assert_eq!(CALLS.with(Cell::get), 4);

    fn explicit(input: Result<(), &str>) -> ErResult<(), Context<Payload>> {
        input.er_val(|v| (v, Payload, "supplied"))
    }

    let tree = explicit(Err("settings")).unwrap_err();
    let Payload = tree.top.data;
    assert_eq!(tree.top.label, "supplied");
    let named = Context::new("settings", Payload, "named");
    let Payload = named.data;
    assert_eq!(named.label, "named");
    assert_eq!(CALLS.with(Cell::get), 4);

    #[derive(Er)]
    struct AllDefault {
        #[er(default)]
        count: u8,
        #[er(default = label())]
        label: String,
    }

    fn empty() -> ErResult<(), AllDefault> {
        Err(fmt::Error).er(())
    }

    fn empty_context(input: Result<(), fmt::Error>) -> ErResult<(), AllDefault> {
        input.er(|_| ())
    }

    fn empty_value(input: Result<(), u8>) -> ErResult<(), AllDefault> {
        input.er_val(|_| ())
    }

    fn empty_borrowed(input: Result<(), fmt::Error>) -> ErResult<(), AllDefault> {
        input.er_with(|_| ())
    }

    fn empty_boundary(input: ErResult<(), Context<()>>) -> Result<(), AllDefault> {
        input.er_into(|_| ())
    }

    let before = CALLS.with(Cell::get);
    assert!(empty_context(Ok(())).is_ok());
    assert!(empty_value(Ok(())).is_ok());
    assert!(empty_borrowed(Ok(())).is_ok());
    assert!(empty_boundary(Ok(())).is_ok());
    assert_eq!(CALLS.with(Cell::get), before);

    for tree in [
        empty_context(Err(fmt::Error)).unwrap_err(),
        empty_value(Err(7)).unwrap_err(),
        empty_borrowed(Err(fmt::Error)).unwrap_err(),
    ] {
        assert_eq!(tree.top.count, 0);
        assert_eq!(tree.top.label, "fallback");
    }
    let error =
        empty_boundary(Err(ErTree::from(Context::new("settings", (), "supplied")))).unwrap_err();
    assert_eq!(error.label, "fallback");
    assert_eq!(CALLS.with(Cell::get), before + 4);

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

    let tree = alias(((7, 85),)).unwrap_err();
    assert_eq!(tree.top.pair, ((7, 85),));
    assert_eq!(tree.top.data, 0);

    let tree = empty().unwrap_err();
    assert_eq!(tree.top.count, 0);
    assert_eq!(tree.top.label, "fallback");

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
    let supplied = Optional::new("settings", None);
    assert!(supplied.value.is_none());
    assert_eq!(supplied.path, "settings");
}

#[derive(Er)]
#[er(wrap)]
struct BailContext {
    path: String,
    #[er(default)]
    retries: u32,
}

#[test]
fn bail() {
    fn tree(path: &str, line: &Cell<u32>) -> ErResult<(), BailContext> {
        line.set(line!() + 1);
        er_bail!(|_| path);
    }

    fn report(path: &str) -> Result<(), ErReport<BailContext>> {
        er_bail!(|_| path);
    }

    fn top(path: &str) -> Result<(), ErTop<BailContext>> {
        er_bail!(|_| path);
    }

    fn wrap(path: &str) -> Result<(), BailContextWrap> {
        er_bail!(|_| path);
    }

    let line = Cell::new(0);
    let tree = tree("settings", &line).unwrap_err();
    #[cfg(feature = "src_locations")]
    assert_eq!(tree.src_location.line(), line.get());

    for tree in [
        tree,
        report("settings").unwrap_err().tree,
        top("settings").unwrap_err().tree,
        wrap("settings").err().unwrap().tree,
    ] {
        assert_eq!(tree.top.path, "settings");
        assert_eq!(tree.top.retries, 0);
        assert!(tree.nodes.is_empty());
    }
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

    fn supplied_unit() -> ErResult<(), Optional> {
        Err(7_u8).er_val::<Optional, ErFields>(|_| ())
    }

    fn supplied(value: Option<()>) -> ErResult<(), Optional> {
        Err(value).er_val(|v| v)
    }

    assert!(omitted().unwrap_err().top.value.is_none());
    assert_eq!(supplied_unit().unwrap_err().top.value, Some(()));
    assert_eq!(supplied(Some(())).unwrap_err().top.value, Some(()));
    assert!(supplied(None).unwrap_err().top.value.is_none());

    let empty: ErResult<(), Optional> = Err(fmt::Error).er(());
    assert!(empty.unwrap_err().top.value.is_none());
}

#[derive(Er)]
#[er(format = "saved")]
struct Saved<T> {
    #[er(into_report_string)]
    diagnostics: String,
    #[er(default)]
    data: T,
}

#[derive(Er)]
#[er(format = "saved")]
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
        input().er_into(|report| report.single_line())
    }

    fn supplied() -> Result<(), Saved<Payload>> {
        input().er_into(|report| (report, Payload))
    }

    fn request() -> Result<(), Request<()>> {
        input().er_into(|report| (report, "request"))
    }

    let err = saved().unwrap_err();
    assert!(err.diagnostics.contains("settings"));
    assert!(!err.diagnostics.contains('\n'));
    let () = err.data;

    let err = supplied().unwrap_err();
    assert!(err.diagnostics.contains(&fmt::Error.to_string()));
    let Payload = err.data;

    let err = request().unwrap_err();
    assert_eq!(err.request, "request");
    assert!(err.diagnostics.contains("settings"));
    let () = err.data;

    let boxed: Result<(), Box<Saved<()>>> = input().er_into(|report| report);
    assert!(boxed.unwrap_err().diagnostics.contains("settings"));

    #[cfg(target_has_atomic = "ptr")]
    {
        let shared: Result<(), std::sync::Arc<Request<()>>> =
            input().er_into(|report| (report, "shared"));
        assert_eq!(shared.unwrap_err().request, "shared");
    }

    let saved: Saved<()> = input()
        .unwrap_err()
        .into_er_report()
        .single_line()
        .er_into(|report| report);
    assert!(!saved.diagnostics.contains('\n'));
}
