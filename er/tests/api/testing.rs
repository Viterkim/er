use core::num::ParseIntError;
use er::*;

#[test]
pub fn nested_helpers() {
    fn inner(line: &mut u32) -> ErTest {
        *line = line!() + 1;
        "aint_even_a_number_cmon_man".parse::<u16>()?;
        Ok(())
    }

    fn outer(line: &mut u32) -> ErTest {
        inner(line)?;
        Ok(())
    }

    let mut line = 0;
    let error = outer(&mut line).unwrap_err();
    assert!(error.tree.er_contains::<ParseIntError>());
    assert_eq!(error.tree.er_find_all::<ErTestError>().count(), 1);
    #[cfg(feature = "src_locations")]
    {
        assert_eq!(error.tree.src_location.file(), file!());
        assert_eq!(error.tree.src_location.line(), line);
    }
    let report = format!("{error:?}");
    assert!(report.starts_with("TestError"));
    assert!(report.contains("invalid digit"));
    assert_eq!(report, error.to_string());
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
}
