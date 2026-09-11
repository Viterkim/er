use er::*;

pub struct NotDebug;

#[derive(Er)]
pub struct VisibleEr {
    pub details: NotDebug,
}

#[derive(Debug)]
pub struct Owned;

#[derive(Er)]
pub struct ConversionEr {
    pub value: Owned,
}

pub fn main() {
    let value = Owned;
    let _ = ConversionEr::new(&value);
}
