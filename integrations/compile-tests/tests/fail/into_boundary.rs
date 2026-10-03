use er::*;

#[derive(Er)]
pub struct RunErr;

#[derive(Er)]
pub struct PublicError {
    #[er(into_top)]
    pub kind: RunErr,
    #[er(into_report_string)]
    pub diagnostics: String,
}
pub fn skip_hook() -> Result<(), PublicError> {
    Err::<(), _>(ErTree::from(RunErr))?;
    Ok(())
}

pub fn main() {}
