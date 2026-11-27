use core::num::ParseIntError;
use er::*;
use std::io;

#[test]
pub fn unit() -> ErTest {
    assert_eq!(Ok::<_, ()>(85).er_test()?, 85);

    let err = Err::<(), _>(()).er_test().unwrap_err();
    assert!(err.tree.nodes.is_empty());

    Ok(())
}

#[test]
pub fn nested_helpers() {
    fn inner() -> ErTest {
        "aint_even_a_number_cmon_man".parse::<u16>().er_test()?;
        Ok(())
    }

    fn outer() -> ErTest {
        inner().er_test()?;
        Ok(())
    }

    let error = outer().unwrap_err();
    assert_eq!(error.er_find_all::<ErTestError>().count(), 1);

    let error = Err::<(), _>(io::Error::other("port").er_save("kept", "report"))
        .er_test()
        .unwrap_err();
    assert_eq!(error.tree.nodes[0].saved[0].title, "kept");
}

#[test]
pub fn tree() {
    let source = ErTree::new(
        std::io::Error::other("ports"),
        [
            "bad".parse::<u16>().unwrap_err(),
            "65536".parse::<u16>().unwrap_err(),
        ],
    );

    let result = (|| -> ErTest {
        Err::<(), _>(source)?;
        Ok(())
    })();

    assert_eq!(result.unwrap_err().tree.nodes[0].nodes.len(), 2);
}

#[test]
pub fn option() {
    let result = (|| -> ErTest {
        None::<u16>.ok_or("expected a port")?;
        Ok(())
    })();
    assert!(result.unwrap_err().to_string().contains("expected a port"));

    let error = None::<u16>.er_test().unwrap_err();
    assert!(error.tree.er_contains::<ErTestOptionNone>());
}

#[test]
pub fn helpers() -> ErTest {
    fn read_port(input: &str) -> ErTest<u16> {
        input.parse().er_test()
    }

    let mut called = false;
    read_port("85").er::<io::Error>(|| {
        called = true;
        io::Error::other("port")
    })?;
    assert!(!called);

    let error = read_port("bad")
        .er::<io::Error>(|| io::Error::other("port"))
        .unwrap_err();
    assert!(error.er_contains::<ParseIntError>());

    let error = read_port("bad")
        .er_with::<io::Error, _>(|err| io::Error::other(err.to_string()))
        .unwrap_err();
    assert_eq!(error.top.to_string(), "TestError");

    let _ = read_port("bad").er_with::<ErTestError, _>(|_| {});

    let error = read_port("bad")
        .er_with_tree::<io::Error, _>(|tree| {
            io::Error::other(tree.er_find::<ParseIntError>().unwrap().to_string())
        })
        .unwrap_err();
    assert!(error.top.to_string().contains("invalid digit"));

    let error = (|| -> ErTest {
        ["bad", "85", "65536"]
            .into_iter()
            .map(read_port)
            .er_collect_all::<Vec<_>, ErTestError>(|| ErTestError)?;
        Ok(())
    })()
    .unwrap_err();
    assert_eq!(error.tree.er_find_all::<ParseIntError>().count(), 2);

    Ok(())
}

#[cfg(feature = "macros")]
#[derive(Er)]
pub struct ChecksErr;
#[cfg(feature = "macros")]
#[test]
pub fn macros() -> ErTest {
    fn read_port(input: &str) -> ErTest<u16> {
        input.parse().er_test()
    }

    let (port, enabled) = er_try!(
        ChecksErr::new,
        [read_port("85"), "true".parse::<bool>().er_test()],
    )?;
    assert_eq!((port, enabled), (85, true));

    let error = (|| -> ErTest {
        let results = vec![read_port("bad"), read_port("65536")];
        er_all!(ChecksErr::new, results)?;
        Ok(())
    })()
    .unwrap_err();
    assert_eq!(error.tree.er_find_all::<ParseIntError>().count(), 2);

    let error = (|| -> ErTest {
        er_all!(|| ErTestError, [read_port("bad"), read_port("65536")])?;
        Ok(())
    })()
    .unwrap_err();
    let error = er_add!(error, [read_port("nope"), read_port("85")]);
    assert_eq!(error.nodes.len(), 3);

    let error = er_try!(ChecksErr::new, [read_port("bad"), read_port("85")]).unwrap_err();
    let failure = read_port("65536").unwrap_err();
    let error = er_add!(error, [failure, read_port("85")]);
    assert_eq!(error.er_find_all::<ParseIntError>().count(), 2);

    let _ = (|| -> ErTest { er_bail!(ChecksErr::new) })();
    let _ = (|| -> ErTest { er_bail!(()) })();

    let error = (|| -> ErTest { er_bail!(|| ErTestError) })().unwrap_err();
    assert!(error.tree.nodes.is_empty());

    let error = (|| -> ErTest { er_bail!(|_| "bad port") })().unwrap_err();
    assert!(error.to_string().contains("bad port"));

    let error = (|| -> ErTest {
        let failure: ErBoxError = Box::new(io::Error::other("bad port"));
        er_bail!((failure, String::from("driver report")))
    })()
    .unwrap_err();
    assert!(error.er_contains::<io::Error>());
    assert!(error.er_report().contains("driver report"));

    Ok(())
}

#[cfg(feature = "macros")]
#[derive(Er)]
pub enum PortErr {
    Missing,
    Bad,
}
#[cfg(feature = "macros")]
#[test]
pub fn assert_err() -> ErTest {
    assert_eq!(er_assert_err!(Err::<(), _>(85_u8)), 85);

    let missing = || Err::<(), _>(ErTree::from(PortErr::Missing));
    er_assert_err!(missing(), PortErr::Missing);

    let error = (|| -> ErTest {
        er_assert_err!(missing(), PortErr::Bad);
        Ok(())
    })()
    .unwrap_err();
    assert!(error.er_contains::<ErTestWrongErr>());

    Ok(())
}
