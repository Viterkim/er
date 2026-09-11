use er::*;

#[derive(ErFormat)]
pub struct Data {
    pub value: u8,
}

#[derive(ErFormat)]
#[er(wrap)]
pub struct Wrapped;

#[derive(ErFormat)]
#[er(crate = ::er)]
pub struct RuntimePath;

#[derive(ErFormat)]
pub struct Exact {
    #[er(exact)]
    pub value: u8,
}

pub fn needs_error(_: impl std::error::Error) {}

pub fn main() {
    let _ = Data::new(7);
    needs_error(Data { value: 7 });
}
