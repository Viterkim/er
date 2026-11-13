use quote::quote;

#[test]
pub fn format_options() -> syn::Result<()> {
    for options in [
        quote!(display = "first", display = "second"),
        quote!(display_alt = "first", display_alt = "second"),
        quote!(debug = "first", debug = "second"),
        quote!(format = "first", format = "second"),
        quote!(display =),
        quote!(display_alt =),
        quote!(display_alt = 85),
        quote!(debug = 85),
        quote!(display = "{unknown}"),
        quote!(display_alt = "{unknown}"),
        quote!(debug = "{unknown}"),
    ] {
        let item = syn::parse2(quote! {
            #[er(#options)]
            struct Example;
        })?;
        assert!(crate::generate::expand(&item).is_err(), "{options}");
    }
    Ok(())
}

#[test]
pub fn named_positions() -> syn::Result<()> {
    for format in [
        "{}",
        "{0}",
        "{00}",
        "{value:0$}",
        "{value..0}",
        "{value.}",
        "{value.0 + 1}",
    ] {
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
    }
    for source in [
        "struct Data { #[er(no_constructors)] value: u8 }",
        "enum Data { #[er(no_constructors)] Value }",
    ] {
        let item = syn::parse_str(source)?;
        let error = crate::generate::expand(&item).unwrap_err();
        assert_eq!(
            error.to_string(),
            "put `no_constructors` on the struct or enum",
            "{source}"
        );
    }
    Ok(())
}

#[test]
pub fn default_options() -> syn::Result<()> {
    for source in [
        "#[er(default)] struct Data;",
        "struct Data { #[er(default, default)] value: u8 }",
        "struct Data { #[er(default =)] value: u8 }",
        "struct Data { #[er(default())] value: u8 }",
        "enum Data { Value(#[er(default)] u8) }",
    ] {
        let item = syn::parse_str(source)?;
        assert!(crate::generate::expand(&item).is_err(), "{source}");
    }
    Ok(())
}

#[test]
pub fn source_options() -> syn::Result<()> {
    for code in [
        "struct Data { #[er(source, source)] cause: u8 }",
        "struct Data { #[er(source = true)] cause: u8 }",
        "struct Data { #[er(source())] cause: u8 }",
        "struct Data { #[er(source(wrong))] cause: u8 }",
        "struct Data { #[er(source(deref, deref))] cause: u8 }",
        "struct Data { #[er(source(deref = true))] cause: u8 }",
        "struct Data { #[er(source(deref), source)] cause: u8 }",
        "struct Data { #[er(show_in_parent)] cause: u8 }",
        "struct Data { #[er(source, show_in_parent, show_in_parent)] cause: u8 }",
        "struct Data { #[er(source, show_in_parent = true)] cause: u8 }",
        "struct Data { #[er(source, show_in_parent, skip)] cause: u8 }",
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
        "struct Data { #[er(into_snapshot)] report: S }",
        "struct Data { #[er(into_report_string)] report: S }",
        "#[er(into_report_string)] struct Data;",
    ] {
        let item = syn::parse_str(code)?;
        assert!(crate::generate::expand(&item).is_err(), "{code}");
    }
    Ok(())
}
