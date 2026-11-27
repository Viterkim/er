use er::*;
use std::process::ExitCode;

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

// main returns ErMain and uses `.er(..)?` like any other function. A failure prints the whole report.
pub fn main() -> ErMain {
    read_port("85").er(())?;
    public("85").er(())?;

    Ok(())
}

// With your own error, its trees go through plain `?`.
pub fn app_main() -> ErMain<ReadPortErr, ExitCode> {
    read_port("85")?;

    Ok(ExitCode::SUCCESS)
}

pub async fn port(input: &str) {
    let _result = async {
        let value = input.parse::<u16>().er(|| ReadPortErr)?;
        ErOk(value)
    }
    .await;
}
