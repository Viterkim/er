use crate::generate::constructors::exact_type;
use quote::quote;
use std::collections::HashSet;

#[test]
pub fn primitive_paths() -> syn::Result<()> {
    let type_names = HashSet::new();
    for path in [
        "u8",
        "std::primitive::u8",
        "::core::primitive::u8",
        "::r#core::r#primitive::u8",
        "r#std::r#primitive::u8",
    ] {
        let ty = syn::parse_str(path)?;
        assert!(exact_type(&ty, &type_names), "{path}");
    }
    Ok(())
}

#[test]
pub fn named_positions() -> syn::Result<()> {
    for format in ["{}", "{0}", "{00}", "{value:0$}"] {
        let item = syn::parse2(quote! {
            #[er(format = #format)]
            struct Named { value: u8 }
        })?;

        assert!(crate::generate::expand(&item).is_err(), "{format}");
    }
    Ok(())
}

#[test]
pub fn position_overflow() -> syn::Result<()> {
    for format in [
        "{9999999999999999999999999999999999999999}",
        "{0:9999999999999999999999999999999999999999$}",
    ] {
        let item = syn::parse2(quote! {
            #[er(format = #format)]
            struct Tuple(u8);
        })?;

        assert!(crate::generate::expand(&item).is_err(), "{format}");
    }
    Ok(())
}
