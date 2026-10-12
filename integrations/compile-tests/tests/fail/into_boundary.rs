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

#[derive(Er)]
pub struct TextError {
    #[er(into_report_string)]
    pub diagnostics: String,
    #[er(default)]
    pub data: (),
}
pub fn skip_text_hook() -> Result<(), TextError> {
    Err::<(), _>(String::from("plain text"))?;
    Ok(())
}

pub fn main() {}
