use er::*;

#[derive(Er)]
pub struct PortEr(pub String);
pub fn read_port(input: &str) -> Er<u16, PortEr> {
    let port: u16 = input.parse().er(|| PortEr::new(input))?;
    Ok(port)
}

#[derive(Er)]
pub struct ReadEr;
pub fn read() -> Er<(), ReadEr> {
    let _port = read_port("fakenumber").er(ReadEr::new)?;
    Ok(())
}

#[derive(Er)]
pub enum ModeEr {
    Missing,
}
pub fn read_mode(mode: Option<&str>) -> Er<&str, ModeEr> {
    let mode = mode.er(ModeEr::missing)?;
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
                "nope",
                Some("microsoftjavaakacsharp")
            ),
            read_config("ComputerKatten", "ComputerKatten_token", "85", None),
        ],
    )?;
    Ok(())
}

pub fn main() {
    if let Err(e) = startup() {
        eprintln!("{}", e.er_report());
    }
}
