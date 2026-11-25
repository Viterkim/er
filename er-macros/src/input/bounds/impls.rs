use super::{BoundSyntax, FormatBounds};
use crate::names::plain;
use proc_macro2::{TokenStream, TokenTree};
use std::collections::HashSet;
use syn::{Ident, Type};

impl<'a, 'out> FormatBounds<'a, 'out> {
    pub fn new(
        name: &'a str,
        parameters: &'a HashSet<String>,
        types: &'out mut Vec<&'a Type>,
        keep_type_bound: bool,
    ) -> Self {
        Self {
            name,
            parameters,
            types,
            uses_parameters: false,
            recursive: false,
            keep_type_bound,
        }
    }
}
impl<'a> syn::visit::Visit<'a> for FormatBounds<'a, '_> {
    fn visit_type(&mut self, ty: &'a Type) {
        if !self.keep_type_bound
            && let Type::Path(path) = ty
            && path.qself.is_none()
        {
            if is_self_path(&path.path, self.name) {
                self.recursive = true;
                return;
            }

            // Bounding the type as a whole loops when it is this type, so bound what's inside it instead,
            // like std's derives. That also holds up when it turns out to be someone else's type.
            if is_qualified_self(&path.path, self.name) {
                self.recursive = true;
                syn::visit::visit_type(self, ty);
                return;
            }
        }

        let start = self.types.len();

        // Keep projections whole, their arguments don't tell us how to format them.
        // Keep for<'a> with the type that declares it too.
        let keep_type_bound = self.keep_type_bound
            || matches!(ty, Type::FnPtr(_) | Type::Ptr(_) | Type::TraitObject(_))
            || matches!(ty, Type::Path(path) if path.qself.is_some());

        // Each type gets its own flags, but shares the collected bounds.
        let mut nested = FormatBounds::new(
            self.name,
            self.parameters,
            &mut *self.types,
            keep_type_bound,
        );
        match ty {
            Type::Verbatim(tokens) => nested.uses_parameters = mentions(tokens, self.parameters),
            _ => syn::visit::visit_type(&mut nested, ty),
        }

        if !nested.recursive {
            // PhantomData<T> can be Debug without T being Debug.
            nested.types.truncate(start);
            if nested.uses_parameters && !self.keep_type_bound {
                nested.types.push(ty);
            }
        }

        self.uses_parameters |= nested.uses_parameters;
        self.recursive |= nested.recursive;
    }

    fn visit_ident(&mut self, ident: &'a Ident) {
        self.uses_parameters |= ident == "Self" || self.parameters.contains(&plain(ident));
    }

    fn visit_lifetime(&mut self, lifetime: &'a syn::Lifetime) {
        self.uses_parameters |= self.parameters.contains(&super::lifetime_name(lifetime));
    }

    fn visit_macro(&mut self, mac: &'a syn::Macro) {
        syn::visit::visit_path(self, &mac.path);
        self.uses_parameters |= mentions(&mac.tokens, self.parameters);
    }

    fn visit_expr(&mut self, expr: &'a syn::Expr) {
        match expr {
            syn::Expr::Verbatim(tokens) => {
                self.uses_parameters |= mentions(tokens, self.parameters)
            }
            _ => syn::visit::visit_expr(self, expr),
        }
    }
}

impl<'a> syn::visit::Visit<'a> for BoundSyntax {
    fn visit_macro(&mut self, _: &'a syn::Macro) {
        self.opaque = true;
    }

    fn visit_type(&mut self, ty: &'a Type) {
        match ty {
            Type::Verbatim(_) => self.opaque = true,
            _ => syn::visit::visit_type(self, ty),
        }
    }

    fn visit_expr(&mut self, expr: &'a syn::Expr) {
        match expr {
            syn::Expr::Verbatim(_) => self.opaque = true,
            _ => syn::visit::visit_expr(self, expr),
        }
    }
}

pub fn is_self_path(path: &syn::Path, name: &str) -> bool {
    if path.leading_colon.is_some() {
        return false;
    }

    match path.segments.len() {
        1 => {
            let ident = &path.segments[0].ident;
            ident == "Self" || plain(ident) == name
        }
        2 => path.segments[0].ident == "self" && plain(&path.segments[1].ident) == name,
        _ => false,
    }
}

/// `crate::Name<T>` or `super::Name<T>`, most likely this type spelled from the crate root. A derive can't see
/// which module it's in, so it can also be another module's type with the same name.
pub fn is_qualified_self(path: &syn::Path, name: &str) -> bool {
    let first = &path.segments[0].ident;

    path.leading_colon.is_none()
        && path.segments.len() > 1
        && (first == "crate" || first == "super")
        && path
            .segments
            .last()
            .is_some_and(|segment| plain(&segment.ident) == name)
}

pub fn mentions(tokens: &TokenStream, parameters: &HashSet<String>) -> bool {
    // A lifetime comes through as `'` and then its name, and only counts with the `'` in front.
    let mut lifetime = false;

    tokens.clone().into_iter().any(|token| {
        let quoted = core::mem::take(&mut lifetime);

        match token {
            TokenTree::Ident(ident) if quoted => {
                parameters.contains(&format!("'{}", plain(&ident)))
            }
            TokenTree::Ident(ident) => ident == "Self" || parameters.contains(&plain(&ident)),
            TokenTree::Group(group) => mentions(&group.stream(), parameters),
            TokenTree::Punct(punct) => {
                lifetime = punct.as_char() == '\'';
                false
            }
            TokenTree::Literal(_) => false,
        }
    })
}
