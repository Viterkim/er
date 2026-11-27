pub mod attrs;
pub mod bounds;
pub mod format;
pub mod impls;

use attrs::{ContainerOptions, FieldOptions};
use format::Formats;
use std::collections::HashSet;
use syn::{DeriveInput, Ident};

pub struct Field<'a> {
    pub item: &'a syn::Field,
    pub options: FieldOptions,
    pub used_in_format: bool,
}

pub struct Case<'a> {
    pub variant: Option<&'a Ident>,
    pub attrs: &'a [syn::Attribute],
    pub shape: &'a syn::Fields,
    pub fields: Vec<Field<'a>>,
    pub formats: Formats,
    pub constructors: Constructors<'a>,
}

#[derive(Clone, Copy)]
pub enum Constructors<'a> {
    TypeVisibility,
    Restricted(&'a syn::Visibility),
    NonExhaustive,
    Disabled,
}

pub struct Input<'a> {
    pub item: &'a DeriveInput,
    pub options: ContainerOptions,
    pub cases: Vec<Case<'a>>,
    pub parameters: HashSet<String>,
    pub names: HashSet<String>,
    pub scope_names: HashSet<String>,
}

#[derive(Default)]
pub struct IdentifierNames {
    pub names: HashSet<String>,
}

// An argument named `size` would hide the function in `[u8; size()]`.
#[derive(Default)]
pub struct ValueNames {
    pub names: HashSet<String>,
}
