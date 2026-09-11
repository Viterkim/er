use er::*;

#[derive(Er)]
#[er(wrap(output = sideways))]
pub struct BadForward;

#[derive(Er)]
pub struct ConflictingFieldOptions {
    #[er(skip, censor)]
    pub value: u8,
}

#[derive(Er)]
#[er(wrap, wrap)]
pub struct DuplicateWrap;

#[derive(Er)]
#[er()]
pub struct EmptyOptions;

#[derive(Er)]
pub struct FieldWithWrap {
    #[er(wrap)]
    pub value: u8,
}

#[derive(Er)]
#[er(skip)]
pub struct TypeWithSkip {
    pub value: u8,
}

#[derive(Er)]
#[er(wrapp)]
pub struct UnknownOption;

#[derive(Er)]
pub enum VariantWithCensor {
    Fine { value: u8 },
    #[er(censor)]
    Misplaced { value: u8 },
}

#[derive(Er)]
pub enum VariantWithUnknown {
    Fine(u8),
    #[er(bogus)]
    Misplaced(u8),
}

pub fn main() {}
