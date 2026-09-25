use er::*;

#[derive(Er)]
pub struct ReadPortErr;
pub fn read_port(input: &str) -> ErResult<u16, ReadPortErr> {
    input.parse().er(())
}

// main accepts the report as its error.
pub fn main() -> Result<(), ErReport<ReadPortErr>> {
    read_port("85").er_report()?;

    Ok(())
}
