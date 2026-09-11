use core::{error::Error, fmt::Error as FmtError, num::ParseIntError};
use er::*;

#[test]
pub fn nested_helpers() {
    fn inner() -> TestEr {
        "aint_even_a_number_cmon_man".parse::<u16>().t_er()?;
        Ok(())
    }

    fn outer() -> TestEr {
        inner().er_tree().t_er()?;
        Ok(())
    }

    let report = outer().unwrap_err();
    assert!(report.er_contains::<ParseIntError>());
    assert_eq!(report.er_find_all::<TestError>().count(), 2);
}

#[test]
pub fn test_errors() -> TestEr {
    assert_eq!(format!("{TestError}"), "TestEr");
    assert_eq!(format!("{TestError:?}"), "TestEr");
    assert_eq!("85".parse::<u32>().t_er()?, 85);

    let report = "aint_even_a_number_cmon_man"
        .parse::<u32>()
        .t_er()
        .unwrap_err();
    assert!(report.er_contains::<ParseIntError>());
    assert!(report.to_string().starts_with("TestEr"));

    let boxed: Result<(), Box<dyn Error + Send + Sync>> = Err(Box::new(FmtError));
    assert!(boxed.t_er().is_err());

    let missing: Option<()> = None;
    assert!(missing.t_er().unwrap_err().tree.nodes.is_empty());
    assert_eq!(Some(7).t_er()?, 7);

    #[derive(Er)]
    pub struct ParseEr;

    let tree = "aint_even_a_number_cmon_man"
        .parse::<u32>()
        .er(ParseEr::new)
        .unwrap_err();
    #[cfg(feature = "src_locations")]
    let src = tree.src_location;
    let result: Er<(), ParseEr> = Err(tree);
    let _line = line!() + 1;
    let report = result.t_er().unwrap_err();

    assert!(report.er_contains::<ParseEr>());
    assert!(report.er_contains::<ParseIntError>());
    #[cfg(feature = "src_locations")]
    {
        assert_eq!(report.tree.src_location.line(), _line);
        assert_eq!(report.tree.nodes[0].src_location, src);
    }

    Ok(())
}
