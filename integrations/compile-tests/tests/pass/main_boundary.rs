use er::*;

#[derive(Er)]
pub struct ReadPortErr;
pub fn read_port(input: &str) -> ErResult<u16, ReadPortErr> {
    input.parse().er(())
}

#[derive(Er)]
pub struct PortError {
    #[er(into_top)]
    pub kind: ReadPortErr,
    #[er(into_report_string)]
    pub report: String,
}
pub fn saved(input: &str) -> Result<(), PortError> {
    read_port(input).er_into::<PortError>(|tree| {
        eprintln!("{}", tree.er_report());
    })?;
    Ok(())
}

// main accepts the report as its error.
pub fn main() -> Result<(), ErReport<ReadPortErr>> {
    read_port("85").er_report()?;

    Ok(())
}
