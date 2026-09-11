use er::*;

#[derive(Er)]
pub struct BackendEr;
pub fn ingest<T>(result: anyhow::Result<T>) -> Er<T, BackendEr> {
    result.er(BackendEr::new)
}

pub fn outward(result: Er<(), BackendEr>) -> anyhow::Result<()> {
    result.er_report().opaque_err()?;
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

    // If you thought i sucked at coming up with names check this out.
    // Gets the original type back, but drops Anyhow's backtrace.
    // `map_err` is fine here because we are converting Anyhow, not throwing away an `ErTree`.
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
