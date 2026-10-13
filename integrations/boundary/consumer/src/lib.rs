#[cfg(test)]
mod tests {
    use boundary_provider::{
        Failure, FunctionError, LocatedError, ManualError, NoConstructors, OpenError, ReadErr,
        Revision,
    };
    use std::{error::Error, io, sync::Arc};

    #[test]
    fn ordinary_errors() {
        let line = line!() + 1;
        let diagnostic = LocatedError::new("banana");
        let location: &'static std::panic::Location<'static> = diagnostic.location;
        assert_eq!((location.file(), location.line()), (file!(), line));

        let err = boundary_provider::read().unwrap_err();
        assert_eq!(format!("{err:?}"), "provider failed, secret *CENSORED*");
        assert!(err.source().unwrap().is::<ReadErr>());

        struct Payload;
        let supplied = Failure::new(
            err.kind,
            err.diagnostics,
            err.single_line,
            Payload,
            "supplied",
        );
        let Payload = supplied.data;

        let denied = OpenError::denied("archive");
        assert_eq!(denied.to_string(), "access denied for archive");

        let failed = OpenError::failed(Some(Arc::new(io::Error::other("disk"))));
        assert!(failed.source().unwrap().is::<io::Error>());

        let _: &dyn Error = &ManualError::new(85);
        let _: &dyn Error = &NoConstructors { code: 7 };

        let err = FunctionError::new(Revision::new(85));
        assert_eq!(err.to_string(), "couldn't save revision 85");
    }
}
