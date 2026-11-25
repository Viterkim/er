use er::*;

#[derive(Er)]
pub struct BackendErr;
pub fn ingest<T>(result: anyhow::Result<T>) -> ErResult<T, BackendErr> {
    result.er(())
}

pub fn outward(result: ErResult<(), BackendErr>) -> anyhow::Result<()> {
    result.er_split_with(|(_, report)| anyhow::anyhow!(report))?;
    Ok(())
}

// A library hands its report out next to its error. On anyhow you keep the report and drop the type.
pub fn from_library(result: Result<(), (BackendErr, String)>) -> anyhow::Result<()> {
    result.map_err(|(_, report)| anyhow::anyhow!(report))
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

        let error = from_library(Err::<(), _>(tree).er_split()).unwrap_err();
        assert!(error.to_string().contains("NativeErr(85)"));

        let tree = ingest::<()>(Err(anyhow::Error::new(NativeErr(85)))).unwrap_err();
        let error = outward(Err(tree)).unwrap_err();
        assert!(error.to_string().contains("NativeErr(85)"));
    }

    // If you thought i sucked at coming up with names check this out.
    // Gets the original type back, but drops Anyhow's backtrace.
    #[test]
    pub fn original_downcast() {
        let result: anyhow::Result<()> = Err(anyhow::Error::new(NativeErr(85)));
        let tree = result
            .map_err(anyhow::Error::reallocate_into_boxed_dyn_error_without_backtrace)
            .er(BackendErr::new)
            .unwrap_err();
        assert_eq!(tree.er_find::<NativeErr>().unwrap().0, 85);
    }
}
