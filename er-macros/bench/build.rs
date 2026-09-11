use std::{env, error::Error, fmt::Write as _, fs, path::PathBuf};

pub fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var_os("CARGO_FEATURE_DERIVE").is_none() {
        return Ok(());
    }

    let mut source = String::new();
    for i in 0..500 {
        writeln!(source, "#[derive(Er)] pub struct Unit{i};")?;
    }
    for i in 0..100 {
        writeln!(
            source,
            "#[derive(Er)] pub struct Named{i} {{ pub path: std::path::PathBuf, pub msg: String }}"
        )?;
    }

    source.push_str("#[derive(Er)] pub enum Large {\n");
    for i in 0..1000 {
        writeln!(source, "Variant{i} {{ input: String }},")?;
    }
    source.push_str("}\n");

    let nested = format!("{}Self{}", "Vec<(T, ".repeat(32), ")>".repeat(32));
    writeln!(
        source,
        "#[derive(Er)] pub struct Recursive<T> {{ pub children: {nested} }}"
    )?;

    let output = PathBuf::from(env::var("OUT_DIR")?);
    fs::write(output.join("errors.rs"), source)?;
    Ok(())
}
