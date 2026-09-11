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

#[test]
pub fn std_error_options() -> syn::Result<()> {
    for options in [
        quote!(std_error),
        quote!(output = top, std_error, std_error),
        quote!(output = top, std_error = true),
        quote!(output = report, std_error()),
    ] {
        let item = syn::parse2(quote! {
            #[er(wrap(#options))]
            struct Example;
        })?;
        assert!(crate::generate::expand(&item).is_err());
    }
    for options in [
        quote!(output = top, std_error),
        quote!(std_error, output = report),
    ] {
        let item = syn::parse2(quote! {
            #[er(wrap(#options))]
            struct Example;
        })?;
        crate::generate::expand(&item)?;
    }
    Ok(())
}

#[test]
pub fn no_constructors_options() -> syn::Result<()> {
    for source in [
        "#[er(no_constructors, no_constructors)] struct Data;",
        "#[er(no_constructors = true)] struct Data;",
        "#[er(no_constructors())] struct Data;",
    ] {
        let item = syn::parse_str(source)?;
        assert!(crate::generate::expand(&item).is_err(), "{source}");
        assert!(crate::format::expand(&item).is_err(), "{source}");
    }
    for source in [
        "struct Data { #[er(no_constructors)] value: u8 }",
        "enum Data { #[er(no_constructors)] Value }",
    ] {
        let item = syn::parse_str(source)?;
        for result in [crate::generate::expand(&item), crate::format::expand(&item)] {
            assert_eq!(
                result.err().map(|error| error.to_string()).as_deref(),
                Some("put `no_constructors` on the struct or enum"),
                "{source}"
            );
        }
    }
    Ok(())
}
