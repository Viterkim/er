use crate::names::plain;
use syn::{Attribute, Error, Ident, LitStr, Meta, Path, Result};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FieldMode {
    Normal,
    Skip,
    Censor,
}

pub struct FieldOptions {
    pub mode: FieldMode,
    pub exact: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WrapOutput {
    Top,
    Report,
}

#[derive(Default)]
pub struct WrapOptions {
    pub name: Option<Ident>,
    pub output: Option<WrapOutput>,
    pub std_error: bool,
}

#[derive(Default)]
pub struct ContainerOptions {
    pub er_path: Option<Path>,
    pub wrap: Option<WrapOptions>,
    pub format: Option<LitStr>,
    pub no_constructors: bool,
}

pub fn er_attributes(attributes: &[Attribute]) -> impl Iterator<Item = &Attribute> {
    attributes.iter().filter(|attribute| {
        attribute
            .path()
            .get_ident()
            .is_some_and(|ident| plain(ident) == "er")
    })
}

pub fn require_list(attribute: &Attribute, expected: &str) -> Result<()> {
    match &attribute.meta {
        Meta::List(list) if list.tokens.is_empty() => Err(Error::new_spanned(
            attribute,
            format!("empty `#[er(...)]`; {expected}"),
        )),
        Meta::List(_) => Ok(()),
        _ => Err(Error::new_spanned(attribute, expected.to_string())),
    }
}

pub fn container(attributes: &[Attribute]) -> Result<ContainerOptions> {
    const EXPECTED: &str = "expected `#[er(format = \"...\")]`, `#[er(crate = ...)]`, \
        `#[er(wrap)]`, or `#[er(no_constructors)]`";

    let mut options = ContainerOptions::default();

    for attribute in er_attributes(attributes) {
        require_list(attribute, EXPECTED)?;

        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("no_constructors") {
                if options.no_constructors {
                    return Err(meta.error("`no_constructors` is set more than once"));
                }
                if !meta.input.is_empty() && !meta.input.peek(syn::token::Comma) {
                    return Err(meta.error("`no_constructors` takes no value"));
                }
                options.no_constructors = true;
                return Ok(());
            }

            if meta.path.is_ident("format") {
                if options.format.is_some() {
                    return Err(meta.error("`format` is set more than once"));
                }
                options.format = Some(meta.value()?.parse()?);
                return Ok(());
            }

            if meta.path.is_ident("crate") {
                if options.er_path.is_some() {
                    return Err(meta.error("`crate` is set more than once"));
                }
                options.er_path = Some(meta.value()?.parse()?);
                return Ok(());
            }

            if meta.path.is_ident("wrap") {
                if options.wrap.is_some() {
                    return Err(meta.error("`wrap` is set more than once"));
                }
                options.wrap = Some(wrap(&meta)?);
                return Ok(());
            }

            if meta.path.is_ident("skip")
                || meta.path.is_ident("censor")
                || meta.path.is_ident("exact")
            {
                return Err(meta.error("put `skip`, `censor`, or `exact` on a field"));
            }

            Err(meta.error(EXPECTED))
        })?;
    }

    Ok(options)
}

pub fn wrap(meta: &syn::meta::ParseNestedMeta<'_>) -> Result<WrapOptions> {
    let mut options = WrapOptions::default();

    if meta.input.peek(syn::token::Eq) {
        return Err(
            meta.error("`wrap` takes no value; use `wrap(name = ...)` or `wrap(output = ...)`")
        );
    }

    if !meta.input.peek(syn::token::Paren) {
        return Ok(options);
    }

    meta.parse_nested_meta(|inner| {
        if inner.path.is_ident("name") {
            if options.name.is_some() {
                return Err(inner.error("`name` is set more than once"));
            }
            options.name = Some(inner.value()?.parse()?);
            return Ok(());
        }

        if inner.path.is_ident("output") {
            if options.output.is_some() {
                return Err(inner.error("`output` is set more than once"));
            }

            let value: Ident = inner.value()?.parse()?;
            let output = match value.to_string().as_str() {
                "top" => WrapOutput::Top,
                "report" => WrapOutput::Report,
                _ => {
                    return Err(Error::new_spanned(
                        &value,
                        "expected `output = top` or `output = report`",
                    ));
                }
            };
            options.output = Some(output);
            return Ok(());
        }

        if inner.path.is_ident("std_error") {
            if options.std_error {
                return Err(inner.error("`std_error` is set more than once"));
            }
            if inner.input.peek(syn::token::Eq) || inner.input.peek(syn::token::Paren) {
                return Err(inner.error("`std_error` takes no value"));
            }
            options.std_error = true;
            return Ok(());
        }

        Err(inner.error("expected `name = ...`, `output = ...`, or `std_error`"))
    })?;

    if options.std_error && options.output.is_none() {
        return Err(meta.error("`std_error` requires `output = top` or `output = report`"));
    }

    Ok(options)
}

pub fn variant(attributes: &[Attribute]) -> Result<Option<LitStr>> {
    let mut format = None;
    for attribute in er_attributes(attributes) {
        require_list(attribute, "expected `#[er(format = \"...\")]`")?;
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("no_constructors") {
                return Err(meta.error("put `no_constructors` on the struct or enum"));
            }
            if !meta.path.is_ident("format") {
                return Err(meta.error(
                    "expected `format = \"...\"`; put `skip`, `censor`, or `exact` on a field, \
                     or `wrap`/`crate` on the enum",
                ));
            }
            if format.is_some() {
                return Err(meta.error("`format` is set more than once"));
            }
            format = Some(meta.value()?.parse()?);
            Ok(())
        })?;
    }

    Ok(format)
}

pub fn field(attributes: &[Attribute]) -> Result<FieldOptions> {
    const EXPECTED: &str = "expected `#[er(skip)]`, `#[er(censor)]`, or `#[er(exact)]`";

    let mut mode = FieldMode::Normal;
    let mut exact = false;

    for attribute in er_attributes(attributes) {
        require_list(attribute, EXPECTED)?;

        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("exact") {
                if exact {
                    return Err(meta.error("`exact` is set more than once"));
                }
                if !meta.input.is_empty() && !meta.input.peek(syn::token::Comma) {
                    return Err(meta.error("`exact` takes no value"));
                }
                exact = true;
                return Ok(());
            }

            let found = if meta.path.is_ident("skip") {
                FieldMode::Skip
            } else if meta.path.is_ident("censor") {
                FieldMode::Censor
            } else if meta.path.is_ident("format") {
                return Err(meta.error("put `format = \"...\"` on the struct or enum variant"));
            } else if meta.path.is_ident("no_constructors") {
                return Err(meta.error("put `no_constructors` on the struct or enum"));
            } else if meta.path.is_ident("wrap") || meta.path.is_ident("crate") {
                return Err(meta.error(
                    "`wrap` and `crate` are type options; put them on the struct or enum, \
                     not on a field",
                ));
            } else {
                return Err(meta.error(EXPECTED));
            };

            if mode != FieldMode::Normal {
                return Err(meta.error("choose only one of `skip` or `censor`"));
            }

            if !meta.input.is_empty() && !meta.input.peek(syn::token::Comma) {
                return Err(meta.error("this option takes no value"));
            }

            mode = found;
            Ok(())
        })?;
    }

    Ok(FieldOptions { mode, exact })
}
