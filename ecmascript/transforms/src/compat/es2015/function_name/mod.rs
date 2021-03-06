use ast::*;
use swc_common::{Fold, FoldWith, Visit, VisitWith};

#[cfg(test)]
mod tests;

/// `@babel/plugin-transform-function-name`
///
/// # Example
/// ## In
/// ```js
/// var number = function (x) {
///   return x;
/// };
/// var Foo = (class {});
/// ```
/// ## Out
/// ```js
/// var number = function number(x) {
///   return x;
/// }
/// var Foo = (class Foo {});
/// ```
pub fn function_name() -> FnName {
    FnName
}

#[derive(Clone, Copy)]
pub struct FnName;

struct Renamer {
    name: Option<Ident>,
}

impl Fold<VarDeclarator> for FnName {
    fn fold(&mut self, decl: VarDeclarator) -> VarDeclarator {
        let mut decl = decl.fold_children(self);

        match decl.name {
            Pat::Ident(ref mut ident) => {
                let mut folder = Renamer {
                    name: Some(ident.clone()),
                };
                let init = decl.init.fold_with(&mut folder);

                return VarDeclarator { init, ..decl };
            }
            _ => decl,
        }
    }
}

impl Fold<AssignExpr> for FnName {
    fn fold(&mut self, expr: AssignExpr) -> AssignExpr {
        let mut expr = expr.fold_children(self);

        if expr.op != op!("=") {
            return expr;
        }

        match expr.left {
            PatOrExpr::Pat(box Pat::Ident(ref mut ident))
            | PatOrExpr::Expr(box Expr::Ident(ref mut ident)) => {
                let mut folder = Renamer {
                    name: Some(ident.clone()),
                };

                let right = expr.right.fold_with(&mut folder);

                return AssignExpr { right, ..expr };
            }
            _ => expr,
        }
    }
}

struct UsageFinder<'a> {
    ident: &'a Ident,
    found: bool,
}

impl<'a> Visit<Ident> for UsageFinder<'a> {
    fn visit(&mut self, i: &Ident) {
        if i.span.ctxt() == self.ident.span.ctxt() && i.sym == self.ident.sym {
            self.found = true;
        }
    }
}

impl<'a> UsageFinder<'a> {
    fn find<N>(ident: &'a Ident, node: &N) -> bool
    where
        N: VisitWith<Self>,
    {
        let mut v = UsageFinder {
            ident,
            found: false,
        };
        node.visit_with(&mut v);
        v.found
    }
}

macro_rules! impl_for {
    ($T:tt) => {
        impl Fold<$T> for Renamer {
            fn fold(&mut self, node: $T) -> $T {
                match node.ident {
                    Some(..) => return node,
                    None => {
                        //
                        let name = match self.name.take() {
                            None => {
                                return $T {
                                    ident: None,
                                    ..node
                                };
                            }
                            Some(name) => name,
                        };
                        // If function's body references the name of variable, we just skip the
                        // function
                        if UsageFinder::find(&name, &node) {
                            // self.name = Some(name);
                            $T {
                                ident: None,
                                ..node
                            }
                        } else {
                            $T {
                                ident: Some(name),
                                ..node
                            }
                        }
                    }
                }
            }
        }
    };
}
impl_for!(FnExpr);
impl_for!(ClassExpr);

macro_rules! noop {
    ($T:tt) => {
        impl Fold<$T> for Renamer {
            /// Don't recurse.
            fn fold(&mut self, node: $T) -> $T {
                node
            }
        }
    };
}

noop!(ObjectLit);
noop!(ArrayLit);
noop!(CallExpr);
noop!(NewExpr);
noop!(BinExpr);
noop!(UnaryExpr);
