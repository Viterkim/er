use er::*;

#[derive(Er)]
pub struct BackendEr;
pub fn ingest<T>(result: anyhow::Result<T>) -> Er<T, BackendEr> {
    result.er(BackendEr::new)
}

pub fn outward(result: Er<(), BackendEr>) -> anyhow::Result<()> {
    result.er_report()?;
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[derive(Er)]
    pub struct NativeEr(pub u8);

    #[test]
    pub fn automatic() {
        // Prints fine, but Anyhow's default box hides this type.
        let bare = ingest::<()>(Err(anyhow::Error::new(NativeEr(85)))).unwrap_err();
        assert!(bare.er_find::<NativeEr>().is_none());
        assert!(bare.er_report().to_string().contains("NativeEr(85)"));

        // With context, we can reach NativeEr through source().
        let error = anyhow::Error::new(NativeEr(85)).context("reading config");
        let tree = ingest::<()>(Err(error)).unwrap_err();
        assert_eq!(tree.er_find::<NativeEr>().unwrap().0, 85);

        let error = outward(Err(tree)).unwrap_err();
        let report = error.to_string();
        assert!(report.contains("reading config"));
        assert!(report.contains("NativeEr(85)"));
    }

    // Anyhow's great long name new box, drops its backtrace.
    // Lets er_find see NativeEr. map_err only converts the Anyhow error here, then .er() makes the tree.
    // Remember to usually never use map_err, as in er it would ruin the tree.
    #[test]
    pub fn original_downcast() {
        let result: anyhow::Result<()> = Err(anyhow::Error::new(NativeEr(85)));
        let tree = result
            .map_err(anyhow::Error::reallocate_into_boxed_dyn_error_without_backtrace)
            .er(BackendEr::new)
            .unwrap_err();
        assert_eq!(tree.er_find::<NativeEr>().unwrap().0, 85);
    }
}
