use syn::Type;

pub mod impls;

// Self still means the original error here.
pub struct ReplaceSelf<'a> {
    pub original: &'a Type,
}
