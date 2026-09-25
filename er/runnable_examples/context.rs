use er::*;
use std::{fs::read_to_string, path::PathBuf};

#[derive(Er)]
pub struct FileErr(pub PathBuf);
pub fn read_file(path: &str) -> Er<String, FileErr> {
    let text = read_to_string(path).er(|_| path)?;
    Ok(text)
}

#[derive(Er)]
pub struct AnalyzeErr;
pub fn analyze() -> Er<String, AnalyzeErr> {
    read_file("/tmp/file.txt").er(())
}

#[derive(Er)]
pub struct PortErr {
    pub invalid_port: String,
}
pub fn read_port(input: &str) -> Er<u16, PortErr> {
    let port: u16 = input.parse().er(|_| input)?;
    Ok(port)
}

#[derive(Er)]
pub enum ModeErr {
    MissingMode,
}
pub fn read_mode(mode: Option<&str>) -> Er<&str, ModeErr> {
    let mode = mode.er(ModeErr::missing_mode)?;
    Ok(mode)
}

#[derive(Er)]
pub struct AuthErr(pub String);
pub fn authenticate(machine: &str, token: &str) -> Er<(), AuthErr> {
    if token == format!("{machine}_token") {
        Ok(())
    } else {
        er_bail!(AuthErr::new(machine));
    }
}

#[derive(Er)]
pub struct ConfigErr {
    pub machine: String,
    #[er(censor)]
    pub token: String,
}
pub fn read_config(
    machine: &str,
    token: &str,
    port: &str,
    mode: Option<&str>,
) -> Er<(), ConfigErr> {
    let e = |_| (machine, token);

    authenticate(machine, token).er(e)?;
    read_port(port).er(e)?;
    read_mode(mode).er(e)?;

    Ok(())
}

#[derive(Er)]
pub struct StartupErr(pub String);
pub fn startup() -> Er<(), StartupErr> {
    er_all!(
        |_| "some config checks failed",
        [
            read_config(
                "HaandboldFuglen",
                "HaandboldFuglen_token",
                "aint_even_a_number_cmon_man",
                Some("microsoftjavaakacsharp")
            ),
            read_config("ComputerKatten", "ComputerKatten_token", "85", None),
        ],
    )?;
    Ok(())
}

#[derive(Er)]
pub enum BingoErr {
    // BingoErr::parse() generated
    Parse { input: String, favorite_number: u32 },
}
pub fn bingo(input: &str) -> Er<u8, BingoErr> {
    input.parse().er(|| BingoErr::parse(input, 85))
}

pub fn main() {
    if let Err(e) = read_file("/tmp/file.txt") {
        eprintln!("{}", e.er_report());
        eprintln!("{}", e.er_top());
    }
    if let Err(e) = analyze() {
        eprintln!("{}", e.er_report());
    }
    if let Err(e) = startup() {
        eprintln!("{}", e.er_report());
        eprintln!("{}", e.er_top());
    }
    if let Err(e) = bingo("aint_even_a_number_cmon_man") {
        eprintln!("{}", e.er_top());
    }
}
