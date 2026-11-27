use super::ReplaceSelf;
use syn::visit_mut::{VisitMut, visit_expr_path_mut, visit_path_mut, visit_type_mut};
use syn::{ExprPath, Path, QSelf, Type};

impl ReplaceSelf<'_> {
    fn associated_path(&mut self, path: &mut Path) -> QSelf {
        path.segments = core::mem::take(&mut path.segments)
            .into_iter()
            .skip(1)
            .collect();
        // Keep the :: after Self when the path becomes <Type>::Assoc.
        path.leading_colon = Some(Default::default());
        visit_path_mut(self, path);

        qualified_self(self.original)
    }
}
impl VisitMut for ReplaceSelf<'_> {
    fn visit_type_mut(&mut self, ty: &mut Type) {
        if let Type::Path(path) = ty
            && path.qself.is_none()
            && starts_with_self(&path.path)
        {
            if path.path.segments.len() == 1 {
                *ty = self.original.clone();
            } else {
                path.qself = Some(self.associated_path(&mut path.path));
            }
            return;
        }

        visit_type_mut(self, ty);
    }

    fn visit_expr_path_mut(&mut self, path: &mut ExprPath) {
        if path.qself.is_none() && starts_with_self(&path.path) && path.path.segments.len() > 1 {
            path.qself = Some(self.associated_path(&mut path.path));
        } else {
            visit_expr_path_mut(self, path);
        }
    }
}

pub fn starts_with_self(path: &Path) -> bool {
    path.segments
        .first()
        .is_some_and(|segment| segment.ident == "Self")
}

pub fn qualified_self(original: &Type) -> QSelf {
    let ty = Box::new(original.clone());
    QSelf {
        lt_token: Default::default(),
        ty,
        position: 0,
        as_token: None,
        gt_token: Default::default(),
    }
}
