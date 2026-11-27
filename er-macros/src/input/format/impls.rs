use super::{
    ArgumentUse, Format, FormatArgument, FormatBody, FormatFallbacks, FormatParser, FormatTrait,
    Formats, Output,
};
use crate::fields::option_inner;
use crate::input::attrs::{FormatOptions, FormatValue};
use crate::input::{Field, attrs::FieldMode};
use crate::names::plain;
use proc_macro2::Span;
use std::collections::HashMap;
use syn::{Error, Ident, LitStr, Result};

impl Formats {
    pub fn parse(options: &FormatOptions, fields: &[Field<'_>]) -> Result<Self> {
        Ok(Self {
            shared: parse_body(options.shared.as_ref(), fields)?,
            display: parse_body(options.display.as_ref(), fields)?,
            display_alt: parse_body(options.display_alt.as_ref(), fields)?,
            debug: parse_body(options.debug.as_ref(), fields)?,
        })
    }

    pub fn fallbacks(&self, output: Output) -> FormatFallbacks<'_> {
        let mut formats = match output {
            Output::Display => [None, self.display.as_ref(), self.shared.as_ref()],
            Output::DisplayAlt => [
                self.display_alt.as_ref(),
                self.display.as_ref(),
                self.shared.as_ref(),
            ],
            Output::Debug => [None, self.debug.as_ref(), self.shared.as_ref()],
        };

        let mut use_default = true;
        for slot in &mut formats {
            if use_default {
                use_default = match slot {
                    Some(FormatBody::Template(format)) => format.optional(),
                    Some(FormatBody::Function(_)) => false,
                    None => true,
                };
            } else {
                *slot = None;
            }
        }

        FormatFallbacks {
            formats,
            use_default,
        }
    }

    pub fn outputs(&self) -> &'static [Output] {
        if self.display_alt.is_some() {
            &[Output::Display, Output::DisplayAlt, Output::Debug]
        } else {
            &[Output::Display, Output::Debug]
        }
    }

    pub fn templates(&self) -> impl Iterator<Item = &Format> {
        [&self.shared, &self.display, &self.display_alt, &self.debug]
            .into_iter()
            .flatten()
            .filter_map(|body| match body {
                FormatBody::Template(format) => Some(format),
                FormatBody::Function(_) => None,
            })
    }
}

pub fn parse_body(value: Option<&FormatValue>, fields: &[Field<'_>]) -> Result<Option<FormatBody>> {
    match value {
        Some(FormatValue::Literal(literal)) => {
            Ok(Some(FormatBody::Template(Format::parse(literal, fields)?)))
        }
        Some(FormatValue::Function(path)) => Ok(Some(FormatBody::Function(path.clone()))),
        None => Ok(None),
    }
}

impl FormatTrait {
    pub fn parse(text: &str, span: Span) -> Result<Self> {
        match text {
            "" => Ok(Self::Display),
            "?" | "x?" | "X?" => Ok(Self::Debug),
            "b" => Ok(Self::Binary),
            "o" => Ok(Self::Octal),
            "x" => Ok(Self::LowerHex),
            "X" => Ok(Self::UpperHex),
            "e" => Ok(Self::LowerExp),
            "E" => Ok(Self::UpperExp),
            "p" => Ok(Self::Pointer),
            _ => Err(Error::new(
                span,
                format!(
                    "don't know the format `{text}`, try `{{field}}`, `{{field:?}}` or one of `x`, `X`, `o`, `b`, `e`, `E`, `p`"
                ),
            )),
        }
    }

    pub fn ident(self) -> Ident {
        let name = match self {
            Self::Display => "Display",
            Self::Debug => "Debug",
            Self::Binary => "Binary",
            Self::Octal => "Octal",
            Self::LowerHex => "LowerHex",
            Self::UpperHex => "UpperHex",
            Self::LowerExp => "LowerExp",
            Self::UpperExp => "UpperExp",
            Self::Pointer => "Pointer",
        };
        Ident::new(name, Span::call_site())
    }
}

impl Format {
    pub fn parse(literal: &LitStr, fields: &[Field<'_>]) -> Result<Self> {
        let mut named_fields = HashMap::new();
        for (index, field) in fields.iter().enumerate() {
            if let Some(name) = &field.item.ident {
                named_fields.insert(plain(name), index);
            }
        }

        let mut parser = FormatParser {
            span: literal.span(),
            fields,
            named_fields,
            next_implicit: 0,
            arguments: Vec::new(),
            requirements: Vec::new(),
        };
        let text = literal.value();
        let mut rest = text.as_str();
        let mut output = String::with_capacity(text.len());

        while let Some(ch) = next_char(&mut rest) {
            if ch == '{' || ch == '}' {
                if rest.starts_with(ch) {
                    next_char(&mut rest);
                    output.push(ch);
                    output.push(ch);
                } else if ch == '{' {
                    parser.placeholder(&mut rest, &mut output)?;
                } else {
                    return Err(Error::new(
                        literal.span(),
                        "there's a `}` without a `{`, write `}}` to print a brace",
                    ));
                }
            } else {
                output.push(ch);
            }
        }
        let literal = LitStr::new(&output, literal.span());
        let arguments = parser.arguments;
        let requirements = parser.requirements;

        Ok(Self {
            literal,
            arguments,
            requirements,
        })
    }

    pub fn optional(&self) -> bool {
        self.arguments.iter().any(|argument| argument.optional)
    }
}

impl FormatParser<'_, '_> {
    pub fn field_index(&mut self, name: &str) -> Result<usize> {
        let index = if name.is_empty() {
            let index = self.next_implicit;
            self.next_implicit += 1;
            index
        } else if name.bytes().all(|ch| ch.is_ascii_digit()) {
            name.parse::<usize>().map_err(|_| {
                Error::new(
                    self.span,
                    "that field number is way too big, nobody has a tuple that long",
                )
            })?
        } else {
            // `{r#type}` names the same field as `type`, the way `format!` takes it.
            let field = name.strip_prefix("r#").unwrap_or(name);

            return self.named_fields.get(field).copied().ok_or_else(|| {
                Error::new(
                    self.span,
                    format!("there's no field `{name}` here, check the spelling"),
                )
            });
        };

        // Numbers only select tuple fields, including implicit {} arguments.
        if self
            .fields
            .get(index)
            .is_some_and(|field| field.item.ident.is_none())
        {
            return Ok(index);
        }

        Err(Error::new(
            self.span,
            format!("there's no field `{index}` here, tuple fields count from 0"),
        ))
    }

    pub fn field_argument(&mut self, name: &str) -> Result<FormatArgument> {
        let mut parts = name.split('.');
        let field = self.field_index(parts.next().unwrap_or_default())?;
        let members = parts
            .map(|part| {
                let mut member = syn::parse_str::<syn::Member>(part).map_err(|_| {
                    Error::new(
                        self.span,
                        format!("can't reach `{name}`, write a field like `{{path}}` or something inside it like `{{details.message}}`"),
                    )
                })?;

                // Point type errors in a nested access at the format string, not the derive.
                match &mut member {
                    syn::Member::Named(ident) => ident.set_span(self.span),
                    syn::Member::Unnamed(index) => index.span = self.span,
                }

                Ok(member)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(FormatArgument {
            field,
            members,
            optional: false,
        })
    }

    pub fn argument(&mut self, mut argument: FormatArgument, usage: ArgumentUse) -> Result<usize> {
        let field_index = argument.field;
        let field = self.fields.get(field_index).ok_or_else(|| {
            Error::new(
                self.span,
                format!("there's no field `{field_index}`, tuple fields count from 0"),
            )
        })?;
        if field.options.mode == FieldMode::Skip
            || (field.options.mode == FieldMode::Censor && matches!(usage, ArgumentUse::Count))
        {
            let name = match &field.item.ident {
                Some(name) => plain(name),
                None => field_index.to_string(),
            };
            let message = if field.options.mode == FieldMode::Skip {
                format!("`{name}` has `skip` on it, take it out of the format or drop the `skip`")
            } else {
                format!("censored field `{name}` cannot set width or precision")
            };
            return Err(Error::new(self.span, message));
        }

        let inside = match usage {
            ArgumentUse::Value(FormatTrait::Debug) => !argument.members.is_empty(),
            ArgumentUse::Value(_) => true,
            ArgumentUse::Count => false,
        };
        argument.optional = inside
            && field.options.mode == FieldMode::Normal
            && option_inner(&field.item.ty).is_some();

        match usage {
            ArgumentUse::Value(format) => {
                if !self.requirements.contains(&(argument.clone(), format)) {
                    self.requirements.push((argument.clone(), format));
                }
            }
            ArgumentUse::Count => (),
        }

        let argument_position = self.arguments.len();
        self.arguments.push(argument);
        Ok(argument_position)
    }

    pub fn placeholder(&mut self, rest: &mut &str, output: &mut String) -> Result<()> {
        let end = rest.find([':', '}']).ok_or_else(|| {
            Error::new(
                self.span,
                "there's a `{` without a `}`, write `{{` to print a brace",
            )
        })?;
        let name = &rest[..end];
        *rest = &rest[end..];

        let mut format_spec = String::new();
        let format = if rest.starts_with(':') {
            self.take_char(rest)?;
            self.format_spec(rest, &mut format_spec)?
        } else {
            FormatTrait::Display
        };
        *rest = rest.trim_start();
        if next_char(rest) != Some('}') {
            return Err(Error::new(
                self.span,
                "close the field with `}`, like `{path}` or `{path:?}`",
            ));
        }

        // In "{:.*}", precision comes before the value.
        let argument = self.field_argument(name.trim_end())?;
        let position = self.argument(argument, ArgumentUse::Value(format))?;

        output.push('{');
        output.push_str(&position.to_string());
        if !format_spec.is_empty() {
            output.push(':');
            output.push_str(&format_spec);
        }
        output.push('}');
        Ok(())
    }

    pub fn format_spec(&mut self, rest: &mut &str, output: &mut String) -> Result<FormatTrait> {
        let mut chars = rest.chars();
        let first = chars.next();
        let second = chars.next();
        if second.is_some_and(|ch| matches!(ch, '<' | '^' | '>')) {
            self.copy_char(rest, output)?;
            self.copy_char(rest, output)?;
        } else if first.is_some_and(|ch| matches!(ch, '<' | '^' | '>')) {
            self.copy_char(rest, output)?;
        }

        if rest.starts_with(['+', '-']) {
            self.copy_char(rest, output)?;
        }
        if rest.starts_with('#') {
            self.copy_char(rest, output)?;
        }
        if rest.starts_with('0') && !rest.starts_with("0$") {
            self.copy_char(rest, output)?;
        }

        self.width_or_precision(rest, output)?;
        if rest.starts_with('.') {
            self.copy_char(rest, output)?;
            if rest.starts_with('*') {
                self.take_char(rest)?;
                let field_index = self.field_index("")?;
                let position = self.argument(
                    FormatArgument {
                        field: field_index,
                        members: Vec::new(),
                        optional: false,
                    },
                    ArgumentUse::Count,
                )?;
                output.push_str(&format!("{position}$"));
            } else if !self.width_or_precision(rest, output)? {
                return Err(Error::new(
                    self.span,
                    "a `.` needs a number after it, like `{value:.2}`",
                ));
            }
        }

        let end = rest
            .find(|ch: char| ch == '}' || ch.is_whitespace())
            .unwrap_or(rest.len());
        let kind = &rest[..end];
        let format = FormatTrait::parse(kind, self.span)?;
        output.push_str(kind);
        *rest = &rest[end..];
        Ok(format)
    }

    pub fn take_char(&self, text: &mut &str) -> Result<char> {
        next_char(text).ok_or_else(|| {
            Error::new(
                self.span,
                "the format stops in the middle of a `{...}`, close it with `}`",
            )
        })
    }

    pub fn copy_char(&self, text: &mut &str, output: &mut String) -> Result<()> {
        let ch = self.take_char(text)?;
        output.push(ch);
        Ok(())
    }

    pub fn width_or_precision(&mut self, rest: &mut &str, output: &mut String) -> Result<bool> {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        let end = if digits > 0 {
            digits
        } else {
            rest.find(|ch: char| matches!(ch, '$' | '.' | '}') || ch.is_whitespace())
                .unwrap_or(rest.len())
        };

        if end > 0 && rest[end..].starts_with('$') {
            let field_index = self.field_index(&rest[..end])?;
            let position = self.argument(
                FormatArgument {
                    field: field_index,
                    members: Vec::new(),
                    optional: false,
                },
                ArgumentUse::Count,
            )?;
            output.push_str(&format!("{position}$"));
            *rest = &rest[end + 1..];
            Ok(true)
        } else if digits > 0 {
            output.push_str(&rest[..digits]);
            *rest = &rest[digits..];
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

pub fn next_char(text: &mut &str) -> Option<char> {
    let ch = text.chars().next()?;
    *text = &text[ch.len_utf8()..];
    Some(ch)
}
