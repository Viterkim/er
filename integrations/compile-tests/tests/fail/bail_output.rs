use er::er_bail;

fn main() {
    let _ = || -> Result<(), _> { er_bail!(std::fmt::Error) };
}
