#[derive(old_er::Er)]
#[er(crate = old_er, format = "{diagnostics}")]
pub struct Failure {
    #[er(into_report_string)]
    pub diagnostics: String,
}
pub fn read() -> Result<u8, Failure> {
    Err(Failure::new("provider rejected the read"))
}
