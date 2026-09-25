use er::*;

#[derive(Er)]
pub struct BackendErr;
pub fn ingest<T>(result: anyhow::Result<T>) -> ErResult<T, BackendErr> {
    result.er(())
}

pub fn outward(result: ErResult<(), BackendErr>) -> anyhow::Result<()> {
    result.er_report().opaque_err()?;
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[derive(Er)]
    pub struct NativeErr(pub u8);

    #[test]
    pub fn automatic() {
        let bare = ingest::<()>(Err(anyhow::Error::new(NativeErr(85)))).unwrap_err();
        assert!(bare.er_report().to_string().contains("NativeErr(85)"));

        let error = anyhow::Error::new(NativeErr(85)).context("reading config");
        let tree = ingest::<()>(Err(error)).unwrap_err();
        assert_eq!(tree.er_find::<NativeErr>().unwrap().0, 85);

        let error = outward(Err(tree)).unwrap_err();
        let report = error.to_string();
        assert!(report.contains("reading config"));
        assert!(report.contains("NativeErr(85)"));
    }

    // If you thought i sucked at coming up with names check this out.
    // Gets the original type back, but drops Anyhow's backtrace.
    // `map_err` is fine here because we are converting Anyhow, not throwing away an `ErTree`.
    #[test]
    pub fn original_downcast() {
        let result: anyhow::Result<()> = Err(anyhow::Error::new(NativeErr(85)));
        let tree = result
            .map_err(anyhow::Error::reallocate_into_boxed_dyn_error_without_backtrace)
            .er::<BackendErr>(())
            .unwrap_err();
        assert_eq!(tree.er_find::<NativeErr>().unwrap().0, 85);
    }
}
