pub mod provider;

#[cfg(test)]
mod tests {
    use super::provider;
    use er::*;

    #[derive(Er)]
    pub struct ReadErr;

    #[test]
    fn versions() {
        let tree: ErTree<ReadErr> = provider::read().er(()).unwrap_err();
        assert!(tree.er_contains::<provider::Failure>());

        let converted: provider::Failure = tree
            .into_er_report()
            .single_line()
            .er_into(|report| report.er_report_string());
        assert!(converted.to_string().contains("provider rejected the read"));
        assert!(converted.to_string().contains("ReadErr"));
        assert!(!converted.to_string().contains('\n'));

        let value: ErResult<u8, ReadErr> = Ok(85);
        let value: Result<u8, provider::Failure> = value.er_into(|_| -> String {
            panic!("successful boundary called the error constructor");
        });
        assert_eq!(value.unwrap(), 85);
    }
}
