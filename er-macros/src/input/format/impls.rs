use super::{ArgumentUse, Format, FormatParser, FormatTrait};
use crate::input::{Field, attrs::FieldMode};
use crate::names::plain;
use proc_macro2::Span;
use std::collections::HashMap;
use syn::{Error, Ident, LitStr, Result};

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
            _ => Err(Error::new(span, format!("unknown format trait `{text}`"))),
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
    pub fn parse(literal: &LitStr, fields: &mut [Field<'_>]) -> Result<Self> {
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
                        "unmatched `}` in format; use `}}` for a literal brace",
                    ));
                }
            } else {
                output.push(ch);
            }
        }
        let literal = LitStr::new(&output, literal.span());
        let arguments = parser.arguments;

        Ok(Self { literal, arguments })
    }
}

impl FormatParser<'_, '_> {
    pub fn field_index(&mut self, name: &str) -> Result<usize> {
        let index = if name.is_empty() {
            let index = self.next_implicit;
            self.next_implicit += 1;
            index
        } else if name.bytes().all(|ch| ch.is_ascii_digit()) {
            name.parse::<usize>()
                .map_err(|_| Error::new(self.span, "field index is too large"))?
        } else {
            return self.named_fields.get(name).copied().ok_or_else(|| {
                Error::new(
                    self.span,
                    format!("no field `{name}` in this format's struct or variant"),
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
            format!("no field `{index}` in this format's struct or variant"),
        ))
    }

    pub fn argument(&mut self, field_index: usize, usage: ArgumentUse) -> Result<usize> {
        let field = self
            .fields
            .get_mut(field_index)
            .ok_or_else(|| Error::new(self.span, format!("no field at index {field_index}")))?;
        if field.options.mode == FieldMode::Skip
            || (field.options.mode == FieldMode::Censor && matches!(usage, ArgumentUse::Count))
        {
            let name = match &field.item.ident {
                Some(name) => plain(name),
                None => field_index.to_string(),
            };
            let message = if field.options.mode == FieldMode::Skip {
                format!("format references skipped field `{name}`")
            } else {
                format!("censored field `{name}` cannot set width or precision")
            };
            return Err(Error::new(self.span, message));
        }

        match usage {
            ArgumentUse::Value(format) => {
                if !field.formats.contains(&format) {
                    field.formats.push(format);
                }
            }
            ArgumentUse::Count => {
                field.sets_width_or_precision = true;
            }
        }

        let argument_position = self.arguments.len();
        self.arguments.push(field_index);
        Ok(argument_position)
    }

    pub fn placeholder(&mut self, rest: &mut &str, output: &mut String) -> Result<()> {
        let end = rest
            .find([':', '}'])
            .ok_or_else(|| Error::new(self.span, "unclosed `{` in format"))?;
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
            return Err(Error::new(self.span, "expected `}` after format argument"));
        }

        // In "{:.*}", precision comes before the value.
        let field_index = self.field_index(name.trim_end())?;
        let position = self.argument(field_index, ArgumentUse::Value(format))?;

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
                let position = self.argument(field_index, ArgumentUse::Count)?;
                output.push_str(&format!("{position}$"));
            } else if !self.width_or_precision(rest, output)? {
                return Err(Error::new(self.span, "expected precision after `.`"));
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
        next_char(text).ok_or_else(|| Error::new(self.span, "unexpected end of format"))
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
            let position = self.argument(field_index, ArgumentUse::Count)?;
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
