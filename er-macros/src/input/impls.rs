use super::{
    Case, Field, Input, attrs,
    attrs::FieldMode,
    format::{Format, FormatTrait},
};
use crate::names::plain;
use std::collections::HashSet;
use syn::{Data, DeriveInput, Ident};

impl<'a> Case<'a> {
    pub fn parse(
        variant: Option<&'a Ident>,
        shape: &'a syn::Fields,
        format: Option<&syn::LitStr>,
    ) -> syn::Result<Self> {
        let mut fields = Vec::with_capacity(shape.len());
        for item in shape {
            let options = attrs::field(&item.attrs)?;
            fields.push(Field {
                item,
                options,
                formats: Vec::new(),
                sets_width_or_precision: false,
            });
        }

        let format = match format {
            Some(literal) => Some(Format::parse(literal, &mut fields)?),
            None => {
                for field in &mut fields {
                    if field.options.mode != FieldMode::Skip {
                        field.formats.push(FormatTrait::Debug);
                    }
                }

                None
            }
        };

        Ok(Self {
            variant,
            shape,
            fields,
            format,
        })
    }
}

impl<'a> Input<'a> {
    pub fn parse(item: &'a DeriveInput) -> syn::Result<Self> {
        let options = attrs::container(&item.attrs)?;
        if let Some(wrap) = &options.wrap
            && let Some(name) = &wrap.name
            && plain(name) == plain(&item.ident)
        {
            return Err(syn::Error::new_spanned(
                name,
                "the wrapper needs a different name from the error type",
            ));
        }

        let cases = match &item.data {
            Data::Struct(data) => vec![Case::parse(None, &data.fields, options.format.as_ref())?],
            Data::Enum(data) => {
                if let Some(format) = &options.format {
                    return Err(syn::Error::new_spanned(
                        format,
                        "put `format = \"...\"` on each enum variant that needs it",
                    ));
                }

                let mut cases = Vec::with_capacity(data.variants.len());
                for variant in &data.variants {
                    let format = attrs::variant(&variant.attrs)?;
                    let case = Case::parse(Some(&variant.ident), &variant.fields, format.as_ref())?;
                    cases.push(case);
                }

                cases
            }
            Data::Union(data) => {
                return Err(syn::Error::new_spanned(
                    data.union_token,
                    "Er supports structs and enums only",
                ));
            }
        };

        let const_names = item
            .generics
            .const_params()
            .map(|param| plain(&param.ident))
            .collect();
        let type_names: HashSet<_> = item
            .generics
            .type_params()
            .map(|param| plain(&param.ident))
            .collect();

        if let Some(wrap) = &options.wrap {
            let name = match &wrap.name {
                Some(name) => plain(name),
                None => format!("{}Wrap", plain(&item.ident)),
            };

            let parameter = item
                .generics
                .type_params()
                .find(|parameter| plain(&parameter.ident) == name);
            if let Some(parameter) = parameter {
                let ident = match &wrap.name {
                    Some(ident) => ident,
                    None => &parameter.ident,
                };

                return Err(syn::Error::new_spanned(
                    ident,
                    format!(
                        "wrapper name `{name}` clashes with a type parameter; \
                         use `wrap(name = ...)` to choose another name"
                    ),
                ));
            }
        }

        Ok(Self {
            item,
            options,
            cases,
            const_names,
            type_names,
        })
    }
}
