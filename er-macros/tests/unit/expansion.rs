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

#[test]
pub fn source_options() -> syn::Result<()> {
    for code in [
        "struct Data { #[er(source, source)] cause: u8 }",
        "struct Data { #[er(source = true)] cause: u8 }",
        "struct Data { #[er(source())] cause: u8 }",
        "#[er(source)] struct Data;",
        "enum Data { #[er(source)] Failed(u8) }",
        "struct Data(#[er(source)] u8, #[er(source)] u8);",
        "enum Data { Failed { #[er(source)] first: u8, #[er(source)] second: u8 } }",
    ] {
        let item = syn::parse_str(code)?;
        assert!(crate::generate::expand(&item).is_err(), "{code}");
    }

    let item = syn::parse_str("struct Data(#[er(source)] std::io::Error);")?;
    assert!(crate::format::expand(&item).is_err());
    Ok(())
}

#[test]
pub fn into_options() -> syn::Result<()> {
    for code in [
        "struct Data { #[er(into_top)] kind: E }",
        "struct Data { #[er(into_top)] kind: E, #[er(into_top)] other: E, #[er(into_snapshot)] diagnostics: S }",
        "struct Data { #[er(into_snapshot, into_report_string)] report: S }",
        "struct Data { #[er(into_snapshot = true)] report: S }",
        "#[er(into_snapshot)] struct Data;",
        "enum Data { First(#[er(into_top)] E, #[er(into_snapshot)] S), Second(#[er(into_top)] E, #[er(into_snapshot)] S) }",
        "enum Data { First(#[er(into_snapshot)] S), Second(#[er(into_snapshot)] S) }",
    ] {
        let item = syn::parse_str(code)?;
        assert!(crate::generate::expand(&item).is_err(), "{code}");
    }

    let item = syn::parse_str("struct Data { #[er(into_snapshot)] diagnostics: S }")?;
    assert!(crate::format::expand(&item).is_err());
    Ok(())
}
