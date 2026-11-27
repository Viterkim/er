#[cfg(test)]
mod tests {
    use er::*;

    #[derive(Er)]
    pub struct ReadErr;

    #[test]
    fn portable_saving() {
        let input: ErResult<u8, boundary_provider::ReadErr> = Ok(85);
        let result = input.er_split_with(|_| -> String { panic!("leaving on Ok") });
        assert_eq!(result.unwrap(), 85);

        let input: ErResult<u8, ReadErr> = boundary_provider::read().er(());
        let report = input.unwrap_err().er_report();
        assert!(report.contains("boundary_provider::ReadErr [saved]"));
    }
}
