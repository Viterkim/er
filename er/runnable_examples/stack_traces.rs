use er::*;
use std::path::{Path, PathBuf};

#[derive(Er)]
pub struct ReadErr {
    pub path: PathBuf,
}

#[derive(Er)]
pub struct ConfigErr;

pub fn read_port(path: &Path, contents: &str) -> ErResult<u16, ReadErr> {
    contents.parse().er(|_| path).er_trace()
}

pub fn load_config() -> ErResult<(), ConfigErr> {
    let files = [("server.conf", "eighty-five"), ("backup.conf", "nope")];
    let results = files.map(|(path, contents)| read_port(Path::new(path), contents));
    er_all!((), results).er_trace()
}

pub fn main() {
    if let Err(error) = load_config() {
        println!("{}", error.er_report());
        for trace in &error.stack_traces {
            println!("\n{trace}");
        }

        println!("\nWhich error each capture belongs to:");
        for trace in &error.stack_traces {
            if let Some(source) = error.er_at_index(trace.error_index) {
                println!("{}: {source}", trace.error_index.0);
            }
        }
    }
}
