use er::Er;

#[derive(Er)]
pub struct Duplicate {
    #[er(exact, exact)]
    pub value: u16,
}

#[derive(Er)]
pub struct Value {
    #[er(exact = true)]
    pub value: u16,
}

#[derive(Er)]
#[er(exact)]
pub struct Container {
    pub port: u16,
}

#[derive(Er)]
pub enum Variant {
    #[er(exact)]
    Port(u16),
}

pub fn main() {}
