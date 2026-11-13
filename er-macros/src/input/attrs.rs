use crate::names::plain;
use syn::{Attribute, Error, Expr, ExprPath, Ident, LitStr, Meta, Path, Result};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FieldMode {
    Normal,
    Skip,
    Censor,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SourceMode {
    Normal,
    Deref,
}

#[derive(Clone, Copy)]
pub struct SourceOptions {
    pub mode: SourceMode,
    pub show_in_parent: bool,
}

pub enum FieldDefault {
    Trait,
    Expression(Expr),
}

pub struct FieldOptions {
    pub mode: FieldMode,
    pub exact: bool,
    pub source: Option<SourceOptions>,
    pub default: Option<FieldDefault>,
    pub src_location: bool,
}

#[derive(Clone)]
pub enum FormatValue {
    Literal(LitStr),
    Function(ExprPath),
}

#[derive(Clone, Default)]
pub struct FormatOptions {
    pub shared: Option<FormatValue>,
    pub display: Option<FormatValue>,
    pub display_alt: Option<FormatValue>,
    pub debug: Option<FormatValue>,
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

pub struct ContainerOptions {
    pub er_path: Path,
    pub wrap: Option<WrapOptions>,
    pub format: FormatOptions,
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
            format!("`#[er()]` is empty, {expected}"),
        )),
        Meta::List(_) => Ok(()),
        _ => Err(Error::new_spanned(attribute, expected.to_string())),
    }
}

pub fn container(attributes: &[Attribute]) -> Result<ContainerOptions> {
    const EXPECTED: &str = "expected `#[er(format = \"...\")]`, `#[er(display = \"...\")]`, \
        `#[er(display_alt = \"...\")]`, `#[er(debug = \"...\")]`, `#[er(crate = ...)]`, \
        `#[er(wrap)]`, or `#[er(no_constructors)]`";

    let mut er_path = None;
    let mut wrap_options = None;
    let mut formats = FormatOptions::default();
    let mut no_constructors = false;

    for attribute in er_attributes(attributes) {
        require_list(attribute, EXPECTED)?;

        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("no_constructors") {
                if no_constructors {
                    return Err(meta.error("`no_constructors` is set more than once"));
                }
                if !meta.input.is_empty() && !meta.input.peek(syn::token::Comma) {
                    return Err(meta.error("`no_constructors` takes no value"));
                }
                no_constructors = true;
                return Ok(());
            }

            if meta.path.is_ident("format")
                || meta.path.is_ident("display")
                || meta.path.is_ident("display_alt")
                || meta.path.is_ident("debug")
            {
                format(&meta, &mut formats)?;
                return Ok(());
            }

            if meta.path.is_ident("crate") {
                if er_path.is_some() {
                    return Err(meta.error("`crate` is set more than once"));
                }
                er_path = Some(meta.value()?.parse()?);
                return Ok(());
            }

            if meta.path.is_ident("wrap") {
                if wrap_options.is_some() {
                    return Err(meta.error("`wrap` is set more than once"));
                }
                wrap_options = Some(wrap(&meta)?);
                return Ok(());
            }

            if meta.path.is_ident("default") || meta.path.is_ident("src_location") {
                return Err(meta.error("put `default` or `src_location` on a field"));
            }

            if meta.path.is_ident("show_in_parent") {
                return Err(meta.error("put `show_in_parent` on a source field"));
            }

            if meta.path.is_ident("skip")
                || meta.path.is_ident("censor")
                || meta.path.is_ident("exact")
                || meta.path.is_ident("source")
            {
                return Err(meta.error("put `skip`, `censor`, `exact`, or `source` on a field"));
            }

            if meta.path.is_ident("into_report_string") {
                return Err(meta.error("`into_report_string` is gone, hand the report out next to your error, `Result<T, (MyError, String)>` from `.er_split()`"));
            }

            Err(meta.error(EXPECTED))
        })?;
    }

    Ok(ContainerOptions {
        er_path: er_path.unwrap_or_else(|| syn::parse_quote!(::er)),
        wrap: wrap_options,
        format: formats,
        no_constructors,
    })
}

pub fn wrap(meta: &syn::meta::ParseNestedMeta<'_>) -> Result<WrapOptions> {
    let mut options = WrapOptions::default();

    if meta.input.peek(syn::token::Eq) {
        return Err(meta.error("write `wrap`, `wrap(name = MyName)` or `wrap(output = report)`"));
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
        return Err(meta.error(
            "`std_error` needs to know what to print, add `output = report` or `output = top`",
        ));
    }

    Ok(options)
}

pub fn format(meta: &syn::meta::ParseNestedMeta<'_>, options: &mut FormatOptions) -> Result<()> {
    let (slot, name) = if meta.path.is_ident("display") {
        (&mut options.display, "display")
    } else if meta.path.is_ident("display_alt") {
        (&mut options.display_alt, "display_alt")
    } else if meta.path.is_ident("debug") {
        (&mut options.debug, "debug")
    } else {
        (&mut options.shared, "format")
    };
    if slot.is_some() {
        return Err(meta.error(format!("`{name}` is set more than once")));
    }
    *slot = Some(format_value(meta.value()?)?);
    Ok(())
}

pub fn format_value(input: syn::parse::ParseStream<'_>) -> Result<FormatValue> {
    if input.peek(LitStr) {
        Ok(FormatValue::Literal(input.parse()?))
    } else {
        let path = input.parse().map_err(|_| {
            Error::new(
                input.span(),
                "give it a format string like `\"couldn't open {path}\"` or a function like `Self::summary`",
            )
        })?;
        Ok(FormatValue::Function(path))
    }
}

pub fn variant(attributes: &[Attribute]) -> Result<FormatOptions> {
    let mut formats = FormatOptions::default();
    for attribute in er_attributes(attributes) {
        require_list(attribute, "expected `#[er(format = \"...\")]`")?;
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("no_constructors") {
                return Err(meta.error("put `no_constructors` on the struct or enum"));
            }
            if !meta.path.is_ident("format") && !meta.path.is_ident("display") && !meta.path.is_ident("display_alt") && !meta.path.is_ident("debug") {
                return Err(meta.error(
                    "expected `format = \"...\"`, `display = \"...\"`, `display_alt = \"...\"`, or `debug = \"...\"`; put `skip`, `censor`, `exact`, or `source` on a field, \
                     or `wrap`/`crate` on the enum",
                ));
            }
            format(&meta, &mut formats)?;
            Ok(())
        })?;
    }

    Ok(formats)
}

pub fn field(attributes: &[Attribute]) -> Result<FieldOptions> {
    const EXPECTED: &str = "expected a field option: `skip`, `censor`, `exact`, `source`, `show_in_parent`, \
        `default`, or `src_location`";

    let mut mode = FieldMode::Normal;
    let mut exact = false;
    let mut source = None;
    let mut show_in_parent = None;
    let mut default = None;
    let mut src_location = false;

    for attribute in er_attributes(attributes) {
        require_list(attribute, EXPECTED)?;

        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("into_top") || meta.path.is_ident("into_snapshot") {
                return Err(meta.error("`into_top` and `into_snapshot` are gone, hand the report out next to your error, `Result<T, (MyError, String)>` from `.er_split()`"));
            }

            if meta.path.is_ident("into_report_string") {
                return Err(meta.error("`into_report_string` is gone, hand the report out next to your error, `Result<T, (MyError, String)>` from `.er_split()`"));
            }

            if meta.path.is_ident("src_location") {
                if src_location {
                    return Err(meta.error("`src_location` is set more than once"));
                }
                if default.is_some() {
                    return Err(meta.error("`src_location` fills itself in where the error is made, it can't have a `default` too"));
                }
                if !meta.input.is_empty() && !meta.input.peek(syn::token::Comma) {
                    return Err(meta.error("`src_location` takes no value"));
                }
                src_location = true;
                return Ok(());
            }

            if meta.path.is_ident("default") {
                if src_location {
                    return Err(meta.error("`src_location` fills itself in where the error is made, it can't have a `default` too"));
                }
                if default.is_some() {
                    return Err(meta.error("`default` is set more than once"));
                }
                default = Some(if meta.input.peek(syn::Token![=]) {
                    FieldDefault::Expression(meta.value()?.parse()?)
                } else {
                    if !meta.input.is_empty() && !meta.input.peek(syn::token::Comma) {
                        return Err(meta.error("use `default` or `default = expression`"));
                    }
                    FieldDefault::Trait
                });
                return Ok(());
            }

            if meta.path.is_ident("source") {
                if source.is_some() {
                    return Err(meta.error("`source` is set more than once"));
                }
                let mut mode = SourceMode::Normal;
                if meta.input.peek(syn::token::Paren) {
                    meta.parse_nested_meta(|inner| {
                        if !inner.path.is_ident("deref") {
                            return Err(inner.error("expected `deref`"));
                        }
                        if mode == SourceMode::Deref {
                            return Err(inner.error("`deref` is set more than once"));
                        }
                        if !inner.input.is_empty() && !inner.input.peek(syn::token::Comma) {
                            return Err(inner.error("`deref` takes no value"));
                        }
                        mode = SourceMode::Deref;
                        Ok(())
                    })?;
                    if mode == SourceMode::Normal {
                        return Err(meta.error("expected `source(deref)`"));
                    }
                }
                if !meta.input.is_empty() && !meta.input.peek(syn::token::Comma) {
                    return Err(meta.error("`source` takes no value"));
                }
                source = Some(mode);
                return Ok(());
            }

            if meta.path.is_ident("show_in_parent") {
                if show_in_parent.is_some() {
                    return Err(meta.error("`show_in_parent` is set more than once"));
                }
                if !meta.input.is_empty() && !meta.input.peek(syn::token::Comma) {
                    return Err(meta.error("`show_in_parent` takes no value"));
                }
                show_in_parent = Some(meta.path.clone());
                return Ok(());
            }

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
            } else if meta.path.is_ident("display")
                || meta.path.is_ident("display_alt")
                || meta.path.is_ident("debug")
            {
                return Err(
                    meta.error("put `display` or `debug` on the struct, enum, or enum variant")
                );
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
                return Err(meta.error("pick `skip` or `censor`, not both"));
            }

            if !meta.input.is_empty() && !meta.input.peek(syn::token::Comma) {
                return Err(meta.error("this option takes no value"));
            }

            mode = found;
            Ok(())
        })?;
    }

    if let Some(flag) = &show_in_parent {
        if source.is_none() {
            return Err(Error::new_spanned(
                flag,
                "`show_in_parent` needs `source` on the same field",
            ));
        }
        if mode == FieldMode::Skip {
            return Err(Error::new_spanned(
                flag,
                "pick `show_in_parent` or `skip`, not both",
            ));
        }
    }

    if src_location && exact {
        return Err(Error::new_spanned(
            attributes.iter().find(|attr| attr.path().is_ident("er")),
            "`src_location` isn't an argument, so there's nothing to make `exact`",
        ));
    }

    let source = source.map(|mode| SourceOptions {
        mode,
        show_in_parent: show_in_parent.is_some(),
    });

    Ok(FieldOptions {
        mode,
        exact,
        source,
        default,
        src_location,
    })
}
