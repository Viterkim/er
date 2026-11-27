pub mod impls;

use syn::Type;

pub struct ReplaceSelf<'a> {
    pub original: &'a Type,
}
