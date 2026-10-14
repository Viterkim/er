pub mod impls;

use super::Field;
use proc_macro2::Span;
use std::collections::HashMap;
use syn::{ExprPath, LitStr, Member};

#[derive(Clone, Copy)]
pub enum Output {
    Display,
    DisplayAlt,
    Debug,
}

#[derive(Default)]
pub struct Formats {
    pub shared: Option<FormatBody>,
    pub display: Option<FormatBody>,
    pub display_alt: Option<FormatBody>,
    pub debug: Option<FormatBody>,
}

pub enum FormatBody {
    Template(Format),
    Function(ExprPath),
}

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
    pub arguments: Vec<FormatArgument>,
    pub requirements: Vec<(FormatArgument, FormatTrait)>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct FormatArgument {
    pub field: usize,
    pub members: Vec<Member>,
}

pub enum ArgumentUse {
    Value(FormatTrait),
    Count,
}

pub struct FormatParser<'a, 'b> {
    pub span: Span,
    pub fields: &'a [Field<'b>],
    pub named_fields: HashMap<String, usize>,
    pub next_implicit: usize,
    pub arguments: Vec<FormatArgument>,
    pub requirements: Vec<(FormatArgument, FormatTrait)>,
}
