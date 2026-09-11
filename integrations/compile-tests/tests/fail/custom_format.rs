use er::*;

#[derive(Er)]
#[er(format = "first", format = "second")]
pub struct Duplicate;

#[derive(Er)]
#[er(format = 7)]
pub struct NotText;

#[derive(Er)]
#[er(format = "enum")]
pub enum EnumFormat { Variant }

#[derive(Er)]
pub struct FieldFormat {
    #[er(format = "field")]
    pub field: u8,
}

#[derive(Er)]
#[er(format = "{missing}")]
pub struct Missing { pub field: u8 }

#[derive(Er)]
#[er(format = "{hidden}")]
pub struct Skipped { #[er(skip)] pub hidden: u8 }

#[derive(Er)]
#[er(format = "{value:secret$}")]
pub struct SecretCount { pub value: u8, #[er(censor)] pub secret: usize }

#[derive(Er)]
#[er(format = "{0:q}")]
pub struct BadTrait(pub u8);

#[derive(Er)]
#[er(format = "{0")]
pub struct OpenBrace(pub u8);

#[derive(Er)]
#[er(format = "bad }")]
pub struct CloseBrace;

#[derive(Er)]
#[er(format = "{1}")]
pub struct MissingPosition(pub u8);

#[derive(Er)]
#[er(format = "{value:.}")]
pub struct MissingPrecision { pub value: f64 }

#[derive(Er)]
pub enum DuplicateVariant {
    #[er(format = "one")]
    #[er(format = "two")]
    Variant,
}

#[derive(Er)]
#[er(format = "{value:width$}")]
pub struct SkippedCount { pub value: u8, #[er(skip)] pub width: usize }

pub fn main() {}
