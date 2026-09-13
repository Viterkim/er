use er::*;

#[derive(Er)]
pub struct ReadPortEr;
pub fn read_port(input: &str) -> Er<u16, ReadPortEr> {
    input.parse().er(ReadPortEr::new)
}

// main accepts the report as its error.
pub fn main() -> Result<(), ErReport<ReadPortEr>> {
    read_port("85").er_report()?;

    Ok(())
}
