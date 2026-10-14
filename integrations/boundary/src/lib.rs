pub mod provider;

#[cfg(test)]
mod tests {
    use super::provider;
    use er::*;

    #[derive(Er)]
    pub struct ReadErr;

    #[test]
    fn portable_saving() {
        fn save() -> Result<u8, boundary_provider::Failure> {
            let input: ErResult<u8, boundary_provider::ReadErr> = Err(std::fmt::Error).er(());
            input.er_into(|report| report.single_line())
        }

        let err = save().unwrap_err();
        assert!(err.diagnostics.contains("ReadErr"));
        assert!(
            err.diagnostics
                .contains("an error occurred when formatting")
        );
        assert!(!err.diagnostics.contains('\n'));
        assert_eq!(err.diagnostics, err.single_line);
        let () = err.data;

        struct Payload;
        let input: ErResult<u8, boundary_provider::ReadErr> = Err(std::fmt::Error).er(());
        let err: Result<u8, std::sync::Arc<boundary_provider::Failure<Payload>>> =
            input.er_into(|report| (report, (Payload, "secret sentinel")));
        let err = err.unwrap_err();
        let Payload = err.data;
        assert!(err.diagnostics.contains("ReadErr"));

        let input: ErResult<u8, boundary_provider::ReadErr> = Ok(85);
        let result: Result<u8, boundary_provider::Failure> =
            input.er_into(|_| -> ErReport<boundary_provider::ReadErr> { panic!("saving on Ok") });
        assert_eq!(result.unwrap(), 85);

        let input: ErResult<u8, ReadErr> = boundary_provider::read().er(());
        assert!(
            input
                .unwrap_err()
                .er_contains::<boundary_provider::Failure>()
        );
    }

    #[test]
    fn frozen_protocol() {
        use boundary_provider::{ReadErr, frozen};

        fn input() -> ErResult<u8, ReadErr> {
            Err(std::fmt::Error).er(())
        }

        fn report() -> Result<u8, frozen::Report> {
            input().er_into(|report| report)
        }

        fn propagate() -> Result<u8, frozen::Report> {
            let value = input().er_into::<frozen::Report, _>(|report| report)?;

            Ok(value)
        }

        fn top() -> Result<u8, frozen::Top> {
            input().er_into(|report| report)
        }

        fn single_line() -> Result<u8, frozen::SingleLine> {
            input().er_into(|report| report)
        }

        fn single_line_top() -> Result<u8, frozen::SingleLineTop> {
            input().er_into(|report| report)
        }

        fn layouts() -> Result<u8, frozen::Layouts> {
            input().er_into(|report| report)
        }

        fn layouts_top() -> Result<u8, frozen::LayoutsTop> {
            input().er_into(|report| report)
        }

        let err = report().err().unwrap();
        assert!(err.diagnostics.contains("ReadErr"));
        assert!(err.diagnostics.contains('\n'));
        assert!(propagate().is_err());

        let err = top().err().unwrap();
        let ReadErr = err.kind;
        let () = err.data;
        assert!(err.diagnostics.contains("ReadErr"));

        struct Payload;
        let result: Result<u8, frozen::Top<Payload>> = input().er_into(|report| (report, Payload));
        let Payload = result.err().unwrap().data;

        let err = single_line().err().unwrap();
        assert!(!err.diagnostics.contains('\n'));
        let err = single_line_top().err().unwrap();
        let ReadErr = err.kind;
        assert!(!err.diagnostics.contains('\n'));

        let err = layouts().err().unwrap();
        assert!(err.diagnostics.contains('\n'));
        assert!(!err.single_line.contains('\n'));
        let err = layouts_top().err().unwrap();
        let ReadErr = err.kind;
        assert!(err.diagnostics.contains('\n'));
        assert!(!err.single_line.contains('\n'));
    }

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
