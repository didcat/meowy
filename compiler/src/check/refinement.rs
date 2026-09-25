use super::{Checker, Constant, Place, Result, Value};
use crate::ast::Span;
use crate::diagnostic::Diagnostic;
use crate::flow::{FALSE, Guard, TRUE};
use crate::hir::{self, Type};

impl Checker {
    pub(crate) fn forget(&mut self, id: usize) {
        let aliases = &self.proofs.aliases;
        let alias = aliases.get(&id).map(|alias| alias.root);
        if alias.is_some()
            && !self.flow.spend(
                (self.tags.len() + self.bools.len())
                    .saturating_mul(aliases.len().checked_ilog2().unwrap_or(0) as usize + 1),
            )
        {
            return;
        }
        let same = |root: usize| {
            root == id
                || alias.is_some_and(|alias| {
                    aliases.get(&root).is_some_and(|other| other.root == alias)
                })
        };
        self.tags.retain(|((root, _), _), _| !same(*root));
        self.bools.retain(|(root, _), _| !same(*root));
    }

    pub(crate) fn forget_field(&mut self, id: usize, fields: &[String], span: Span) -> Result<()> {
        let width = fields
            .iter()
            .fold(1usize, |width, name| width.saturating_add(name.len()));
        let aliases = &self.proofs.aliases;
        let alias = aliases.get(&id).map(|alias| alias.root);
        let width = width.saturating_add(if alias.is_some() {
            aliases.len().checked_ilog2().unwrap_or(0) as usize + 1
        } else {
            0
        });
        if !self
            .flow
            .spend((self.tags.len() + self.bools.len()).saturating_mul(width))
        {
            return Err(Diagnostic::unsupported(
                "field refinement budget exhausted",
                span,
            ));
        }
        let affected = |root: usize, path: &[String]| {
            if root == id {
                path.starts_with(fields) || fields.starts_with(path)
            } else {
                alias.is_some_and(|alias| {
                    aliases.get(&root).is_some_and(|other| other.root == alias)
                })
            }
        };
        self.tags
            .retain(|((root, path), _), _| !affected(*root, path));
        self.bools.retain(|(root, path), _| !affected(*root, path));
        Ok(())
    }

    pub(crate) fn forget_mutable(&mut self) {
        let ids: Vec<_> = self
            .scopes
            .iter()
            .flat_map(|scope| scope.values.values())
            .filter_map(|value| match value {
                Value::Local { id, .. } if self.proofs.variable(*id) => Some(*id),
                _ => None,
            })
            .collect();
        for id in ids {
            self.forget(id);
        }
    }

    pub(crate) fn place(value: &hir::Expr) -> Option<Place> {
        match &value.kind {
            hir::ExprKind::Local(id) => Some((*id, Vec::new())),
            hir::ExprKind::Coerce { value } => Self::place(value),
            hir::ExprKind::Field { value, index } => {
                let Type::Record { fields, .. } = &value.ty else {
                    return None;
                };
                let mut place = Self::place(value)?;
                place.1.push(fields[*index].name.clone());
                Some(place)
            }
            _ => None,
        }
    }

    pub(crate) fn variants(&mut self, place: Place, ty: &Type) -> Vec<(Type, Guard)> {
        let key = (place, ty.clone());
        if let Some(tags) = self.tags.get(&key) {
            return tags.clone();
        }
        let mut rest = TRUE;
        let mut tags = Vec::new();
        let count = ty.members().len();
        for (index, ty) in ty.members().iter().enumerate() {
            let guard = if index + 1 == count {
                rest
            } else {
                let tag = self.flow.fresh();
                let guard = self.flow.and(rest, tag);
                let absent = self.flow.not(tag);
                rest = self.flow.and(rest, absent);
                guard
            };
            tags.push((ty.clone(), guard));
        }
        self.tags.insert(key, tags.clone());
        tags
    }

    pub(crate) fn refined(&mut self, place: Place, ty: &Type) -> Type {
        if self.reach == FALSE || !matches!(ty, Type::Union(_)) {
            return ty.clone();
        }
        let types = self
            .variants(place, ty)
            .into_iter()
            .filter_map(|(ty, guard)| self.flow.overlap(self.reach, guard).then_some(ty))
            .collect::<Vec<_>>();
        Type::union(types)
    }

    pub(crate) fn coerce(value: hir::Expr, ty: Type) -> hir::Expr {
        Self::coercion(value, ty).1
    }

    pub(crate) fn coercion(value: hir::Expr, ty: Type) -> (bool, hir::Expr) {
        if value.ty == ty {
            (false, value)
        } else {
            (
                true,
                hir::Expr {
                    span: value.span,
                    ty,
                    kind: hir::ExprKind::Coerce {
                        value: Box::new(value),
                    },
                },
            )
        }
    }

    pub(crate) fn narrow(&mut self, value: hir::Expr) -> Result<hir::Expr> {
        if let Some(place) = Self::place(&value) {
            let ty = self.refined(place.clone(), &value.ty);
            if matches!(value.ty, Type::Union(_)) && self.proofs.variable(place.0) {
                crate::borrow_contract::type_weight(&value.ty, &mut self.flow, value.span)?;
                let tags = self.variants(place, &value.ty);
                self.flow.spend(tags.len() + 1);
                self.proofs
                    .observations
                    .insert((value.span.start, value.span.end), tags);
            }
            Ok(Self::coerce(value, ty))
        } else {
            Ok(value)
        }
    }

    pub(crate) fn storage_type(value: &hir::Expr) -> &Type {
        match &value.kind {
            hir::ExprKind::Coerce { value } => Self::storage_type(value),
            _ => &value.ty,
        }
    }

    pub(crate) fn guard(&mut self, expr: &hir::Expr) -> Guard {
        let key = (expr.span.start, expr.span.end);
        if let Some(guard) = self.guards.get(&key) {
            return *guard;
        }
        let guard = if self.derived_expr(expr) {
            self.flow.fresh()
        } else if let Some(Constant::Bool(value)) = self.constant(expr) {
            if value { TRUE } else { FALSE }
        } else {
            match &expr.kind {
                hir::ExprKind::Unary { op, value } if op == "!" => {
                    let guard = self.guard(value);
                    self.flow.not(guard)
                }
                hir::ExprKind::Binary {
                    op, left, right, ..
                } if op == "&&" || op == "||" => {
                    let left = self.guard(left);
                    let right = self.guard(right);
                    if op == "&&" {
                        self.flow.and(left, right)
                    } else {
                        self.flow.or(left, right)
                    }
                }
                hir::ExprKind::TypeTest { value, ty } => {
                    if ty.accepts(&value.ty) {
                        TRUE
                    } else if ty.intersection(&value.ty) == Type::Never {
                        FALSE
                    } else if let Some(place) = Self::place(value) {
                        let tags = self.variants(place, Self::storage_type(value));
                        let mut guard = FALSE;
                        for (variant, tag) in tags {
                            if ty.accepts(&variant) {
                                guard = self.flow.or(guard, tag);
                            }
                        }
                        guard
                    } else {
                        self.flow.fresh()
                    }
                }
                _ => {
                    if let Some(place) = Self::place(expr) {
                        if let Some(guard) = self.bools.get(&place) {
                            *guard
                        } else {
                            let guard = self.flow.fresh();
                            self.bools.insert(place, guard);
                            guard
                        }
                    } else {
                        self.flow.fresh()
                    }
                }
            }
        };
        self.guards.insert(key, guard);
        guard
    }
}
