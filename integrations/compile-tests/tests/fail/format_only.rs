use er::*;

#[derive(ErFormat)]
#[er(no_constructors)]
pub struct Data {
    pub value: u8,
}

#[derive(ErFormat)]
#[er(wrap)]
pub struct Wrapped;

#[derive(ErFormat)]
#[er(crate = ::er)]
pub struct RuntimePath;

#[derive(Er)]
#[er(no_constructors)]
pub enum NoVariants {
    Missing,
}

pub fn needs_error(_: impl std::error::Error) {}

pub fn main() {
    let _ = Data::new(7);
    let _ = NoVariants::missing();
    needs_error(Data { value: 7 });
}
