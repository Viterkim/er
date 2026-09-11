use attrs::{ContainerOptions, FieldOptions};
use format::{Format, FormatTrait};
use std::collections::HashSet;
use syn::{DeriveInput, Ident};

pub mod attrs;
pub mod bounds;
pub mod format;
pub mod impls;

pub struct Field<'a> {
    pub item: &'a syn::Field,
    pub options: FieldOptions,
    pub formats: Vec<FormatTrait>,
    pub sets_width_or_precision: bool,
}

pub struct Case<'a> {
    pub variant: Option<&'a Ident>,
    pub shape: &'a syn::Fields,
    pub fields: Vec<Field<'a>>,
    pub format: Option<Format>,
}

pub struct Input<'a> {
    pub item: &'a DeriveInput,
    pub options: ContainerOptions,
    pub cases: Vec<Case<'a>>,
    pub const_names: HashSet<String>,
    pub type_names: HashSet<String>,
}
