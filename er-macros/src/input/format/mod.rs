use super::Field;
use proc_macro2::Span;
use std::collections::HashMap;
use syn::LitStr;

pub mod impls;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormatTrait {
    Display,
    Debug,
    Binary,
    Octal,
    LowerHex,
    UpperHex,
    LowerExp,
    UpperExp,
    Pointer,
}

pub struct Format {
    pub literal: LitStr,
    pub arguments: Vec<usize>,
}

pub enum ArgumentUse {
    Value(FormatTrait),
    Count,
}

pub struct FormatParser<'a, 'b> {
    pub span: Span,
    pub fields: &'a mut [Field<'b>],
    pub named_fields: HashMap<String, usize>,
    pub next_implicit: usize,
    pub arguments: Vec<usize>,
}
