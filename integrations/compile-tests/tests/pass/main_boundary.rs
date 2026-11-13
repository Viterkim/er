use er::*;

#[derive(Er)]
pub struct ReadPortErr;
pub fn read_port(input: &str) -> ErResult<u16, ReadPortErr> {
    input.parse().er(())
}

// `?` right after `.er_split()` hands the pair on.
pub fn public(input: &str) -> Result<(), (ReadPortErr, String)> {
    read_port(input).er_split()?;
    Ok(())
}

// main returns ErMain, `?` takes the error and a failure prints the whole report.
pub fn main() -> ErMain {
    read_port("85")?;

    Ok(())
}
