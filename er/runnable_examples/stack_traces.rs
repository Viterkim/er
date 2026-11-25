use er::*;
use std::{
    backtrace::Backtrace,
    path::{Path, PathBuf},
};

#[derive(Er)]
pub struct ReadErr {
    pub path: PathBuf,
}

#[derive(Er)]
pub struct ConfigErr;

pub fn read_port(path: &Path, contents: &str) -> ErResult<u16, ReadErr> {
    contents
        .parse()
        .er(|_| ReadErr::new(path).er_save("stack trace", Backtrace::force_capture))
}

pub fn load_config() -> ErResult<(), ConfigErr> {
    let files = [("server.conf", "eighty-five"), ("backup.conf", "nope")];
    let results = files.map(|(path, contents)| read_port(Path::new(path), contents));
    er_all!((), results)
}

pub fn main() {
    if let Err(error) = load_config() {
        println!("{}", error.er_report());
    }
}
