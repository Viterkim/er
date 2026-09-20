use er::*;
use std::io;

fn read() -> ErLazy {
    Err::<(), _>(io::Error::other("disk broke")).er(())?;
    Ok(())
}

fn pass_through() -> ErLazy {
    read()?;
    Ok(())
}

fn with_context() -> ErLazy {
    read().er(|_| "loading config")?;
    Ok(())
}

#[test]
fn lazy_context() {
    let plain = pass_through().unwrap_err();
    assert_eq!(plain.er_entries().count(), 2);
    assert_eq!(plain.top.to_string(), "failed");

    let contextual = with_context().unwrap_err();
    assert_eq!(contextual.top.to_string(), "loading config");
    assert!(contextual.er_contains::<io::Error>());
}
