pub mod impls;

use syn::Type;

// Self still means the original error here.
pub struct ReplaceSelf<'a> {
    pub original: &'a Type,
}
