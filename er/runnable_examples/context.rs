use er::*;
use std::{fs::read_to_string, path::PathBuf};

#[derive(Er)]
pub struct FileEr(pub PathBuf);
pub fn read_file(path: &str) -> Er<String, FileEr> {
    let text = read_to_string(path).er(|| FileEr::new(path))?;
    Ok(text)
}

#[derive(Er)]
pub struct AnalyzeEr;
pub fn analyze() -> Er<String, AnalyzeEr> {
    read_file("/tmp/file.txt").er(AnalyzeEr::new)
}

#[derive(Er)]
pub struct PortEr {
    pub invalid_port: String,
}
pub fn read_port(input: &str) -> Er<u16, PortEr> {
    let port: u16 = input.parse().er(|| PortEr::new(input))?;
    Ok(port)
}

#[derive(Er)]
pub enum ModeEr {
    MissingMode,
}
pub fn read_mode(mode: Option<&str>) -> Er<&str, ModeEr> {
    let mode = mode.er(ModeEr::missing_mode)?;
    Ok(mode)
}

#[derive(Er)]
pub struct AuthEr(pub String);
pub fn authenticate(machine: &str, token: &str) -> Er<(), AuthEr> {
    if token == format!("{machine}_token") {
        Ok(())
    } else {
        Err(AuthEr::new(machine).er())
    }
}

#[derive(Er)]
pub struct ConfigEr {
    pub machine: String,
    #[er(censor)]
    pub token: String,
}
pub fn read_config(machine: &str, token: &str, port: &str, mode: Option<&str>) -> Er<(), ConfigEr> {
    let er = || ConfigEr::new(machine, token);

    authenticate(machine, token).er(er)?;
    read_port(port).er(er)?;
    read_mode(mode).er(er)?;

    Ok(())
}

#[derive(Er)]
pub struct StartupEr;
pub fn startup() -> Er<(), StartupEr> {
    er_all!(
        StartupEr::new,
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
pub enum BingoEr {
    // BingoEr::parse() generated
    Parse { input: String, favorite_number: u32 },
}
pub fn bingo(input: &str) -> Er<u8, BingoEr> {
    input.parse().er(|| BingoEr::parse(input, 85))
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
