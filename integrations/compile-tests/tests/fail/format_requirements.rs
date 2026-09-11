use er::*;

#[derive(Debug)]
pub struct NotDisplay;

#[derive(Er)]
#[er(format = "{value}")]
pub struct DisplayField { pub value: NotDisplay }

#[derive(Er)]
#[er(format = "{value:width$}")]
pub struct CountField { pub value: u8, pub width: String }

#[derive(Er)]
#[er(format = "{value:x}")]
pub struct GenericField<T> { pub value: T }

pub fn main() {
    let error = GenericField::new("not a number");
    println!("{error}");
}
