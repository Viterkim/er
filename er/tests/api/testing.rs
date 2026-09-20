use core::num::ParseIntError;
use er::*;

#[test]
pub fn nested_helpers() {
    fn inner() -> ErTest {
        "aint_even_a_number_cmon_man".parse::<u16>().er(())?;
        Ok(())
    }

    fn outer() -> ErTest {
        inner().er(())?;
        Ok(())
    }

    let report = outer().unwrap_err();
    assert!(report.er_contains::<ParseIntError>());
    assert_eq!(report.er_find_all::<TestError>().count(), 2);
    assert!(report.to_string().starts_with("ErTest"));
}
