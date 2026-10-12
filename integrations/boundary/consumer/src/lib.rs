#[cfg(test)]
mod tests {
    use boundary_provider::{
        Failure, FunctionError, ManualError, NoConstructors, OpenError, Revision,
    };
    use std::{error::Error, sync::Arc};

    #[test]
    fn ordinary_errors() {
        let line = line!() + 1;
        let diagnostic = boundary_provider::LocatedError::new("banana");
        let location: &'static std::panic::Location<'static> = diagnostic.location;
        assert_eq!(location.line(), line);
        assert!(diagnostic.to_string().contains("banana"));

        let err = boundary_provider::read().unwrap_err();
        let ordinary: &dyn Error = &err;
        assert_eq!(ordinary.to_string(), "provider failed");
        assert_eq!(format!("{err:?}"), "provider failed, secret *CENSORED*");
        assert_eq!(format!("{err:#}"), err.diagnostics);
        assert_eq!(err.secret, "secret sentinel");
        assert!(!err.single_line.contains('\n'));
        assert!(std::ptr::eq(
            ordinary
                .source()
                .unwrap()
                .downcast_ref::<boundary_provider::ReadErr>()
                .unwrap(),
            &err.kind
        ));

        struct Payload;
        let supplied = Failure::new(
            err.kind,
            err.diagnostics,
            err.single_line,
            Payload,
            "supplied",
        );
        let Payload = supplied.data;
        assert_eq!(supplied.secret, "supplied");

        let denied = OpenError::denied("archive");
        assert_eq!(denied.to_string(), "access denied for archive");
        assert_eq!(format!("{denied:?}"), "OpenError::Denied(\"archive\")");

        let cause = Arc::new(std::io::Error::other("disk sentinel"));
        let failed = OpenError::failed(Some(Arc::clone(&cause)));
        let found = failed
            .source()
            .unwrap()
            .downcast_ref::<std::io::Error>()
            .unwrap();
        assert!(std::ptr::eq(found, &*cause));
        assert!(OpenError::failed(None).source().is_none());

        let manual = ManualError::new(85);
        let _: &dyn Error = &manual;
        assert_eq!(manual.code, 85);
        let _: &dyn Error = &NoConstructors { code: 7 };

        let err = FunctionError::new(Revision::new(85));
        let _: &dyn Error = &err;
        assert_eq!(err.to_string(), "couldn't save revision 85");
        assert_eq!(
            format!("{err:?}"),
            "FunctionError { revision: Revision(85) }"
        );
    }
}
