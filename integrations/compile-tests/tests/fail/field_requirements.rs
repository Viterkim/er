use er::*;

pub struct NotDebug;

#[derive(Er)]
pub struct VisibleErr {
    pub details: NotDebug,
}

#[derive(Debug)]
pub struct Owned;

#[derive(Er)]
pub struct ConversionErr {
    pub value: Owned,
}

pub fn main() {
    let value = Owned;
    let _ = ConversionErr::new(&value);
}
