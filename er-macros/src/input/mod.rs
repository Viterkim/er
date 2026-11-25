pub mod attrs;
pub mod bounds;
pub mod format;
pub mod impls;

use attrs::{ContainerOptions, FieldOptions};
use format::{FormatTrait, Formats};
use std::collections::HashSet;
use syn::{DeriveInput, Ident};

pub struct Field<'a> {
    pub item: &'a syn::Field,
    pub options: FieldOptions,
    pub formats: Vec<FormatTrait>,
    pub used_in_format: bool,
}

pub struct Case<'a> {
    pub variant: Option<&'a Ident>,
    pub attrs: &'a [syn::Attribute],
    pub shape: &'a syn::Fields,
    pub fields: Vec<Field<'a>>,
    pub formats: Formats,
}

pub struct Input<'a> {
    pub item: &'a DeriveInput,
    pub options: ContainerOptions,
    pub cases: Vec<Case<'a>>,
    pub const_names: HashSet<String>,
    pub type_names: HashSet<String>,
    pub names: HashSet<String>,
    pub scope_names: HashSet<String>,
}

#[derive(Default)]
pub struct IdentifierNames {
    pub names: HashSet<String>,
}

/// Values a field type names, like `size` in `[u8; size()]`, which a constructor argument with that name would
/// hide. Lifetimes, type paths and modules live in other namespaces, so `'src` or `config::Config` don't count.
#[derive(Default)]
pub struct ValueNames {
    pub names: HashSet<String>,
}
