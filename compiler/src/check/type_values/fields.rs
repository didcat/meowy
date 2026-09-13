use crate::ast::{self, ExprKind};
use crate::check::{
    Checker, Result, Value,
    inputs::{Input, MAX_RECORD_DEPTH, Sources},
};
use crate::diagnostic::Diagnostic;
use crate::hir::Type;

impl Checker {
    pub(crate) fn required_path(&mut self, expr: &ast::Expr) -> Result<(usize, Type, Vec<usize>)> {
        let mut root = expr;
        let mut names = Vec::new();
        loop {
            match &root.kind {
                ExprKind::Field { value, name } => {
                    if names.len() == MAX_RECORD_DEPTH + 1 {
                        return Err(super::Work::budget(expr.span));
                    }
                    names.push((name, root.span));
                    root = value;
                }
                ExprKind::Group(value) => root = value,
                _ => break,
            }
        }
        let ExprKind::Name(root_name) = &root.kind else {
            return Err(Diagnostic::unsupported(
                "computed field roots outside named records",
                expr.span,
            ));
        };
        let (id, ty, limit) = match self.required_value(root_name, root.span)? {
            Value::Local {
                id,
                ty,
                mutable: false,
                ..
            } => (id, ty, MAX_RECORD_DEPTH),
            Value::FileModule { id, ty } => (id, ty, MAX_RECORD_DEPTH + 1),
            _ => {
                return Err(Diagnostic::unsupported(
                    "computed fields outside immutable local records or file exports",
                    expr.span,
                ));
            }
        };
        if names.len() > limit {
            return Err(super::Work::budget(expr.span));
        }
        let mut current = &ty;
        let mut path = Vec::new();
        for (name, span) in names.into_iter().rev() {
            let Type::Record { fields, .. } = current else {
                return Err(Diagnostic::unsupported(
                    "computed field paths outside records",
                    span,
                ));
            };
            if !self.flow.spend(fields.len() + name.len() + 1) {
                return Err(super::Work::budget(span));
            }
            let (index, field) = fields
                .iter()
                .enumerate()
                .find(|(_, field)| field.name == *name)
                .ok_or_else(|| {
                    Self::error("E201", format!("unknown record field `{name}`"), span)
                })?;
            path.push(index);
            current = &field.ty;
        }
        Ok((id, current.clone(), path))
    }

    pub(crate) fn required_field(&mut self, expr: &ast::Expr) -> Result<(Type, Input)> {
        let (id, ty, path) = self.required_path(expr)?;
        if !matches!(ty, Type::Int { .. }) {
            return Err(Diagnostic::unsupported(
                "computed record paths without an integer leaf",
                expr.span,
            ));
        }
        let input = self
            .field_input(id, &path, &Sources::default())
            .ok_or_else(|| {
                Self::error(
                    "E211",
                    "record initializer is unavailable during required type evaluation",
                    expr.span,
                )
            })?;
        Ok((ty, input))
    }
}
