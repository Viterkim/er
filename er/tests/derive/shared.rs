use er::*;
use std::{cell::Cell, io, path::PathBuf, sync::Arc};

#[derive(Debug)]
pub struct Context {
    pub path: PathBuf,
}

#[derive(Er)]
pub struct ReadErr {
    pub context: ErShared<Context>,
}

#[derive(Er)]
pub struct ConfigErr {
    pub context: ErShared<Context>,
    pub machine: String,
}

#[derive(Er)]
pub struct StartupErr;

#[test]
pub fn shared_context() {
    let calls = Cell::new(0);
    let context = |_| {
        calls.set(calls.get() + 1);
        Context {
            path: PathBuf::from("config.toml"),
        }
    };
    let ok: ErResult<(), ReadErr> = Ok::<_, io::Error>(()).er(context);
    assert!(ok.is_ok());
    assert_eq!(calls.get(), 0);

    let result: ErResult<(), ReadErr> = Err(io::Error::other("failed to read")).er(context);
    assert_eq!(calls.get(), 1);
    let tree: ErTree<ConfigErr> = result
        .er_with(|old| (old.context.clone(), "ComputerKatten"))
        .unwrap_err();
    let old = tree.er_find::<ReadErr>().unwrap();

    assert!(ErShared::ptr_eq(&tree.top.context, &old.context));
    assert!(std::ptr::eq(
        tree.top.context.as_ref(),
        old.context.as_ref(),
    ));
    assert_eq!(tree.top.context.path, PathBuf::from("config.toml"));
    assert_eq!(
        old.to_string(),
        "ReadErr { context: Context { path: \"config.toml\" } }"
    );

    let tree = tree.er(StartupErr::new);
    let report = tree.er_report().to_string();
    assert_eq!(report.matches("config.toml").count(), 2);
    assert_eq!(tree.er_report().to_string(), report);
    assert_eq!(tree.er_snapshot().er_report().to_string(), report);

    let context = tree.er_find::<ReadErr>().unwrap().context.clone();
    drop(tree);
    assert_eq!(context.path, PathBuf::from("config.toml"));
    assert_eq!(Arc::strong_count(&context.value), 1);
}

#[derive(Er)]
pub struct PathErr {
    pub path: ErShared<PathBuf>,
}

#[test]
pub fn constructors() {
    let path = PathBuf::from("config.toml");
    let result: ErResult<(), PathErr> = Err(io::Error::other("failed to read")).er(|_| &path);
    let error = result.unwrap_err();
    assert_eq!(error.top.path.as_ref(), &path);
    let next = PathErr::new(&error.top.path);
    assert!(ErShared::ptr_eq(&error.top.path, &next.path));

    let context = Arc::new(Context { path });
    let borrowed = ReadErr::new(&context);
    assert!(Arc::ptr_eq(&context, &borrowed.context.value));
    let owned = ReadErr::new(context);
    assert!(ErShared::ptr_eq(&borrowed.context, &owned.context));

    let text: Arc<str> = Arc::from("config.toml");
    let borrowed: ErShared<str> = (&text).into();
    let owned: ErShared<str> = text.into();
    assert!(ErShared::ptr_eq(&borrowed, &owned));
    assert_eq!(owned.to_string(), "config.toml");
}
