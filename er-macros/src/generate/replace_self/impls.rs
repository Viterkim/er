use super::ReplaceSelf;
use syn::fold::{Fold, fold_expr_path, fold_type};
use syn::{ExprPath, Path, QSelf, Type};

impl ReplaceSelf<'_> {
    pub fn associated_path(&mut self, mut path: Path) -> (QSelf, Path) {
        path.segments = path.segments.into_iter().skip(1).collect();
        let path = self.fold_path(path);
        let qualified = qualified_self(self.original);
        (qualified, path)
    }
}
impl Fold for ReplaceSelf<'_> {
    fn fold_type(&mut self, ty: Type) -> Type {
        let Type::Path(mut path) = ty else {
            return fold_type(self, ty);
        };

        if path.qself.is_none() && starts_with_self(&path.path) {
            if path.path.segments.len() == 1 {
                return self.original.clone();
            }

            let (qualified, tail) = self.associated_path(path.path);
            path.qself = Some(qualified);
            path.path = tail;
            return Type::Path(path);
        }

        fold_type(self, Type::Path(path))
    }

    fn fold_expr_path(&mut self, mut path: ExprPath) -> ExprPath {
        if path.qself.is_none() && starts_with_self(&path.path) && path.path.segments.len() > 1 {
            let (qualified, tail) = self.associated_path(path.path);
            path.qself = Some(qualified);
            path.path = tail;
            return path;
        }

        fold_expr_path(self, path)
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
