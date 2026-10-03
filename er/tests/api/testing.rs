use core::num::ParseIntError;
use er::*;
use std::io;

#[test]
pub fn nested_helpers() {
    fn inner(line: &mut u32) -> ErTest {
        *line = line!() + 1;
        "aint_even_a_number_cmon_man".parse::<u16>().er_test()?;
        Ok(())
    }

    fn outer(line: &mut u32) -> ErTest {
        inner(line).er_test()?;
        Ok(())
    }

    let mut line = 0;
    let error = outer(&mut line).unwrap_err();
    assert!(error.er_contains::<ParseIntError>());
    assert_eq!(error.er_find_all::<ErTestError>().count(), 1);

    #[cfg(feature = "src_locations")]
    {
        assert_eq!(error.tree.src_location.file(), file!());
        assert_eq!(error.tree.src_location.line(), line);
    }

    let report = format!("{error:?}");
    assert!(report.starts_with("TestError"));
    assert!(report.contains("invalid digit"));
    assert_eq!(report, error.to_string());
    assert_eq!(report, error.er_report().to_string());

    let snapshot = error.er_snapshot();
    let error = Err::<(), _>(error.into_er_tree()).er_test().unwrap_err();
    assert_eq!(error.er_snapshot(), snapshot);

    #[cfg(feature = "stack_traces")]
    {
        let _trace_line = line!() + 1;
        let error = Err::<(), _>(error).er_trace().unwrap_err();
        assert_eq!(error.tree.stack_traces.len(), 1);
        assert_eq!(error.tree.stack_traces[0].error_index, ErErrorIndex(0));
        assert_eq!(
            error.tree.stack_traces[0].trace_location.line(),
            _trace_line
        );

        let error = Err::<(), _>(error.into_er_report()).er_test().unwrap_err();
        assert_eq!(error.tree.er_snapshot(), snapshot);
        assert_eq!(error.tree.stack_traces[0].error_index, ErErrorIndex(0));
        assert_eq!(
            error.tree.stack_traces[0].trace_location.line(),
            _trace_line
        );

        #[cfg(feature = "src_locations")]
        assert_eq!(error.tree.src_location.line(), line);
    }
}

#[test]
pub fn tree_and_report() {
    for report in [false, true] {
        let source = ErTree::new(
            std::io::Error::other("ports"),
            [
                "bad".parse::<u16>().unwrap_err(),
                "65536".parse::<u16>().unwrap_err(),
            ],
        );

        #[cfg(feature = "src_locations")]
        let location = source.src_location;

        let mut _line = 0;
        let result = (|| -> ErTest {
            if report {
                _line = line!() + 1;
                Err::<(), _>(source.into_er_report())?;
            } else {
                _line = line!() + 1;
                Err::<(), _>(source)?;
            }
            Ok(())
        })();

        let error = result.unwrap_err();
        assert_eq!(error.tree.nodes.len(), 1);
        assert_eq!(error.tree.nodes[0].nodes.len(), 2);
        assert_eq!(error.tree.er_find_all::<ParseIntError>().count(), 2);
        assert_eq!(
            error.tree.er_find::<std::io::Error>().unwrap().to_string(),
            "ports"
        );

        #[cfg(feature = "src_locations")]
        {
            assert_eq!(error.tree.src_location.line(), _line);
            assert_eq!(error.tree.nodes[0].src_location, location);
        }
    }
}

#[test]
pub fn option() {
    assert_eq!(Some(85).er_test().unwrap(), 85);

    let mut _line = 0;
    let result = (|| -> ErTest {
        _line = line!() + 1;
        None::<u16>.ok_or("expected a port")?;
        Ok(())
    })();

    let error = result.unwrap_err();
    assert!(error.to_string().contains("expected a port"));

    #[cfg(feature = "src_locations")]
    assert_eq!(error.tree.src_location.line(), _line);

    let _line = line!() + 1;
    let error = None::<u16>.er_test().unwrap_err();
    assert!(error.tree.er_contains::<ErTestOptionNone>());
    assert_eq!(error.tree.er_find_all::<ErTestError>().count(), 1);
    assert!(error.to_string().contains("Option was None"));

    #[cfg(feature = "src_locations")]
    assert_eq!(error.tree.src_location.line(), _line);
}

#[test]
pub fn helpers() -> ErTest {
    fn read_port(input: &str) -> ErTest<u16> {
        input.parse().er_test()
    }

    let mut called = false;
    let port = read_port("85").er::<io::Error>(|| {
        called = true;
        io::Error::other("port")
    })?;
    assert_eq!(port, 85);
    assert!(!called);

    let error = read_port("bad")
        .er::<io::Error>(|| io::Error::other("port"))
        .unwrap_err();
    assert!(error.er_contains::<ParseIntError>());
    assert_eq!(error.top.to_string(), "port");

    let error = read_port("bad")
        .er_with(|old| io::Error::other(old.to_string()))
        .unwrap_err();
    assert_eq!(error.top.to_string(), "TestError");
    assert!(error.er_contains::<ParseIntError>());

    let error = read_port("bad")
        .er_with_tree(|old| io::Error::other(old.er_find::<ParseIntError>().unwrap().to_string()))
        .unwrap_err();
    assert!(error.top.to_string().contains("invalid digit"));
    assert!(error.er_contains::<ParseIntError>());

    let mut ports = ["bad", "85", "65536"].into_iter().map(read_port);
    let error = ports
        .by_ref()
        .er_collect::<Vec<_>, ErTestError>(|| ErTestError)
        .unwrap_err();
    assert_eq!(error.er_find_all::<ParseIntError>().count(), 1);
    assert_eq!(ports.next().unwrap()?, 85);

    let ports = ["7", "8"]
        .into_iter()
        .map(read_port)
        .er_collect_all::<Vec<_>, ErTestError>(|| ErTestError)?;
    assert_eq!(ports, [7, 8]);

    let error = (|| -> ErTest {
        ["bad", "85", "65536"]
            .into_iter()
            .map(read_port)
            .er_collect_all::<Vec<_>, ErTestError>(|| ErTestError)?;
        Ok(())
    })()
    .unwrap_err();
    assert_eq!(error.tree.er_find_all::<ParseIntError>().count(), 2);
    assert_eq!(error.tree.er_find_all::<ErTestError>().count(), 3);

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
    assert_eq!(error.tree.er_find_all::<ErTestError>().count(), 3);
    assert!(error.tree.er_contains::<ChecksErr>());

    let error = (|| -> ErTest {
        er_all!(|| ErTestError, [read_port("bad"), read_port("65536")])?;
        Ok(())
    })()
    .unwrap_err();
    assert_eq!(error.tree.nodes.len(), 2);
    assert_eq!(error.tree.er_find_all::<ErTestError>().count(), 3);

    let error = er_add!(error, [read_port("nope"), read_port("85")]);
    assert_eq!(error.nodes.len(), 3);
    assert_eq!(error.er_find_all::<ParseIntError>().count(), 3);

    let error = er_try!(ChecksErr::new, [read_port("bad"), read_port("85")]).unwrap_err();
    let failure = read_port("65536").unwrap_err();
    let error = er_add!(error, [failure, read_port("85")]);
    assert_eq!(error.er_find_all::<ParseIntError>().count(), 2);

    let error = (|| -> ErTest { er_bail!(ChecksErr::new) })().unwrap_err();
    assert!(error.tree.er_contains::<ChecksErr>());
    assert_eq!(error.tree.er_find_all::<ErTestError>().count(), 1);

    let error = (|| -> ErTest { er_bail!(|| ErTestError) })().unwrap_err();
    assert!(error.tree.nodes.is_empty());

    let error = (|| -> ErTest { er_bail!(|_| "bad port") })().unwrap_err();
    assert!(error.to_string().contains("bad port"));

    let _line = line!() + 1;
    let error = (|| -> ErTest { er_bail!(()) })().unwrap_err();
    assert!(error.tree.nodes.is_empty());

    #[cfg(feature = "src_locations")]
    assert_eq!(error.tree.src_location.line(), _line);

    Ok(())
}
