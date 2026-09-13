use crate::ast::{self, ExprKind};
use crate::check::{
    Checker, Result, Value,
    inputs::{Input, MAX_RECORD_DEPTH, Record, Sources},
};
use crate::diagnostic::Diagnostic;
use crate::hir::Type;

pub(crate) enum Source {
    Local(usize),
    Record(Box<Record>),
}

impl Source {
    pub(crate) fn integer(&self, checker: &mut Checker, path: &[usize]) -> Option<Input> {
        match self {
            Self::Local(id) => checker.field_input(*id, path, &Sources::default()),
            Self::Record(input) => input.field(path),
        }
    }

    pub(crate) fn boolean(&self, checker: &mut Checker, path: &[usize]) -> Option<Input<bool>> {
        match self {
            Self::Local(id) => checker.boolean_field_input(*id, path, &Sources::default()),
            Self::Record(input) => input.boolean(path),
        }
    }
    pub(crate) fn record(&self, checker: &mut Checker, path: &[usize]) -> Option<Record> {
        match self {
            Self::Local(id) => {
                let source = checker.input_path(*id, path)?;
                let mut input = checker
                    .record_inputs
                    .get(&source.id)?
                    .project(&source.path)?;
                input.input.work = input.input.work.saturating_add(source.work);
                Some(input)
            }
            Self::Record(input) => input.project(path),
        }
    }
}

impl Checker {
    pub(crate) fn required_path(&mut self, expr: &ast::Expr) -> Result<(Source, Type, Vec<usize>)> {
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
            } => (Source::Local(id), ty, MAX_RECORD_DEPTH),
            Value::FileModule { id, ty } => (Source::Local(id), ty, MAX_RECORD_DEPTH + 1),
            Value::Record { ty, input } => (Source::Record(input), ty, MAX_RECORD_DEPTH),
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
        let input = id.integer(self, &path).ok_or_else(|| {
            Self::error(
                "E211",
                "record initializer is unavailable during required type evaluation",
                expr.span,
            )
        })?;
        Ok((ty, input))
    }
}
