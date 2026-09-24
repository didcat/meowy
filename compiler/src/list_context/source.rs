use crate::ast::{self, ExprKind, StmtKind};
use crate::check::{Checker, Result, Scope, Value};
use crate::diagnostic::Diagnostic;
use crate::flow::Guard;
use crate::hir::Type;

use super::Fit;

impl Checker {
    pub(crate) fn list_shadowed(&mut self, value: &ast::Expr, names: &[&str]) -> Result<bool> {
        if names.is_empty() {
            return Ok(false);
        }
        let mut pending = vec![value];
        while let Some(value) = pending.pop() {
            if pending.len() + 2 > super::MAX_SCALAR_NODES || !self.flow.spend(1) {
                return Err(Diagnostic::unsupported(
                    "record probe scope budget exhausted",
                    value.span,
                ));
            }
            match &value.kind {
                ExprKind::Name(name) => {
                    if !self.flow.spend(names.len().saturating_mul(name.len() + 1)) {
                        return Err(Diagnostic::unsupported(
                            "record probe scope budget exhausted",
                            value.span,
                        ));
                    }
                    if names.contains(&name.as_str()) {
                        return Ok(true);
                    }
                }
                ExprKind::Int(_) | ExprKind::Float(_) => {}
                ExprKind::String(parts)
                    if parts
                        .iter()
                        .all(|part| matches!(part, ast::StringPart::Text(_))) => {}
                ExprKind::Group(value)
                | ExprKind::Unary { value, .. }
                | ExprKind::Field { value, .. } => pending.push(value),
                ExprKind::Binary { left, right, .. } => {
                    pending.push(left);
                    pending.push(right);
                }
                ExprKind::Index { value, index } => {
                    pending.push(value);
                    pending.push(index);
                }
                ExprKind::List(values) => {
                    if pending.len().saturating_add(values.len()) > super::MAX_SCALAR_NODES
                        || !self.flow.spend(values.len())
                    {
                        return Err(Diagnostic::unsupported(
                            "record probe scope budget exhausted",
                            value.span,
                        ));
                    }
                    pending.extend(values);
                }
                ExprKind::Call { callee, args } => {
                    if pending.len().saturating_add(args.len()).saturating_add(1)
                        > super::MAX_SCALAR_NODES
                        || !self.flow.spend(args.len() + 1)
                    {
                        return Err(Diagnostic::unsupported(
                            "record probe scope budget exhausted",
                            value.span,
                        ));
                    }
                    pending.push(callee);
                    pending.extend(args);
                }
                ExprKind::Block(block) if block.label.is_none() => {
                    if pending.len().saturating_add(block.stmts.len()) > super::MAX_SCALAR_NODES
                        || !self.flow.spend(block.stmts.len())
                    {
                        return Err(Diagnostic::unsupported(
                            "record probe scope budget exhausted",
                            value.span,
                        ));
                    }
                    for stmt in &block.stmts {
                        let StmtKind::Emit { value, .. } = &stmt.kind else {
                            return Ok(true);
                        };
                        pending.push(value);
                    }
                }
                _ => return Ok(true),
            }
        }
        Ok(false)
    }

    pub(crate) fn list_assigns(expected: &Type, actual: &Type) -> bool {
        expected.accepts(actual)
            || matches!(actual, Type::Record { primary, .. }
            if !matches!(expected, Type::Record { .. }) && expected.accepts(primary))
    }

    pub(crate) fn list_independent(&mut self, value: &ast::Expr) -> bool {
        match &value.kind {
            ExprKind::Name(_)
            | ExprKind::Call { .. }
            | ExprKind::Field { .. }
            | ExprKind::Index { .. }
            | ExprKind::Ascribe { .. }
            | ExprKind::Dispatch { .. } => true,
            ExprKind::Group(value) => self.list_independent(value),
            ExprKind::Unary { op, .. } if ["&", "*", "!"].contains(&op.as_str()) => true,
            ExprKind::Binary { .. } | ExprKind::Unary { .. } => self.hint(value).is_some(),
            _ => false,
        }
    }

    pub(crate) fn list_symbol<'a>(scopes: &'a [Scope], name: &str) -> Option<&'a Value> {
        scopes.iter().rev().find_map(|scope| scope.values.get(name))
    }

    pub(crate) fn list_source<'a>(
        scopes: &'a [Scope],
        value: &ast::Expr,
    ) -> (Option<&'a Type>, usize) {
        match &value.kind {
            ExprKind::Name(name) => (
                match Self::list_symbol(scopes, name) {
                    Some(Value::Local { ty, .. }) => Some(ty),
                    _ => None,
                },
                1,
            ),
            ExprKind::Call { callee, .. } => {
                let ExprKind::Name(name) = &callee.kind else {
                    return (None, 1);
                };
                (
                    match Self::list_symbol(scopes, name) {
                        Some(Value::Function { result, .. }) => result.as_ref(),
                        _ => None,
                    },
                    1,
                )
            }
            ExprKind::Group(value) => Self::list_source(scopes, value),
            ExprKind::Unary { op, value } if op == "*" => {
                let (ty, work) = Self::list_source(scopes, value);
                (
                    if let Some(Type::Reference(ty)) = ty {
                        Some(ty.as_ref())
                    } else {
                        None
                    },
                    work.saturating_add(1),
                )
            }
            ExprKind::Field { value, name } => {
                let (ty, work) = Self::list_source(scopes, value);
                let ty = if let Some(Type::Reference(ty)) = ty {
                    Some(ty.as_ref())
                } else {
                    ty
                };
                let Some(Type::Record { fields, .. }) = ty else {
                    return (None, work.saturating_add(1));
                };
                if let Some((index, field)) = fields
                    .iter()
                    .enumerate()
                    .find(|(_, field)| &field.name == name)
                {
                    (Some(&field.ty), work.saturating_add(index + 1))
                } else {
                    (None, work.saturating_add(fields.len()))
                }
            }
            ExprKind::Index { value, .. } => {
                let (ty, work) = Self::list_source(scopes, value);
                let ty = if let Some(Type::Reference(ty)) = ty {
                    Some(ty.as_ref())
                } else {
                    ty
                };
                (
                    if let Some(Type::List { element, .. }) = ty {
                        Some(element.as_ref())
                    } else {
                        None
                    },
                    work.saturating_add(1),
                )
            }
            _ => (None, 1),
        }
    }

    pub(crate) fn list_probe(
        &mut self,
        value: &ast::Expr,
        expected: &Type,
        reach: Guard,
    ) -> Result<Fit> {
        crate::borrow_contract::type_weight(expected, &mut self.flow, value.span)?;
        if !self
            .flow
            .spend(value.span.end.saturating_sub(value.span.start) + 1)
        {
            return Err(Diagnostic::unsupported(
                "list candidate work budget exhausted",
                value.span,
            ));
        }
        let scalar = match &value.kind {
            ExprKind::Int(text) => Some(self.integer(text, false, Some(expected), value.span)),
            ExprKind::Float(text) => Some(Self::floating(text, Some(expected), value.span)),
            ExprKind::Unary { op, value: inner } if op == "-" => match &inner.kind {
                ExprKind::Int(text) => Some(self.integer(text, true, Some(expected), value.span)),
                ExprKind::Float(text) => Some(Self::floating(text, Some(expected), value.span)),
                _ => None,
            },
            ExprKind::String(parts)
                if parts
                    .iter()
                    .all(|part| matches!(part, ast::StringPart::Text(_))) =>
            {
                return Ok(if expected.accepts(&Type::String) {
                    Fit::Yes
                } else {
                    Fit::No
                });
            }
            _ => None,
        };
        if let Some(result) = scalar {
            return Ok(match result {
                Ok(value) if expected.accepts(&value.ty) => Fit::Yes,
                Err(error) if error.code == "E207" => Fit::Unknown,
                _ => Fit::No,
            });
        }
        if matches!(
            value.kind,
            ExprKind::Unary { .. }
                | ExprKind::Binary { .. }
                | ExprKind::Call { .. }
                | ExprKind::Dispatch { .. }
        ) && let Some(fit) = self.list_scalar_probe(value, expected, reach)?
        {
            return Ok(fit);
        }
        match &value.kind {
            ExprKind::Group(value) => return self.list_probe(value, expected, reach),
            ExprKind::List(values) => {
                let mut matches = Vec::new();
                for ty in expected.members() {
                    let Type::List { element, capacity } = ty else {
                        continue;
                    };
                    if values.len() > *capacity {
                        continue;
                    }
                    let mut fit = Fit::Yes;
                    for value in values {
                        fit = fit.and(self.list_probe(value, element, reach)?);
                    }
                    if fit != Fit::No {
                        matches.push(fit);
                    }
                }
                return Ok(match matches.as_slice() {
                    [] => Fit::No,
                    [fit] => *fit,
                    _ => Fit::Unknown,
                });
            }
            ExprKind::Block(block) => return self.list_block_probe(block, expected, reach),
            _ => {}
        }
        if let ExprKind::Name(name) = &value.kind
            && matches!(
                Self::list_symbol(&self.scopes, name),
                Some(Value::Constant(_))
            )
            && let Some(ty) = self.literal_default(value)
        {
            return Ok(if expected.accepts(&ty) {
                Fit::Yes
            } else {
                Fit::No
            });
        }
        let (ty, work) = Self::list_source(&self.scopes, value);
        if !self.flow.spend(work) {
            return Err(Diagnostic::unsupported(
                "list source lookup budget exhausted",
                value.span,
            ));
        }
        if let Some(ty) = ty {
            crate::borrow_contract::type_weight(ty, &mut self.flow, value.span)?;
            return Ok(if Self::list_assigns(expected, ty) {
                Fit::Yes
            } else if ty
                .members()
                .iter()
                .any(|ty| Self::list_assigns(expected, ty))
            {
                Fit::Unknown
            } else {
                Fit::No
            });
        }
        Ok(Fit::Unknown)
    }

    pub(crate) fn list_block_probe(
        &mut self,
        block: &ast::Block,
        expected: &Type,
        reach: Guard,
    ) -> Result<Fit> {
        if block.label.is_some() {
            return Ok(Fit::Unknown);
        }
        let mut fields = Vec::new();
        for stmt in &block.stmts {
            let StmtKind::Emit {
                label: None,
                name,
                ty: None,
                mutable,
                value,
            } = &stmt.kind
            else {
                return Ok(Fit::Unknown);
            };
            if !self.flow.spend(fields.len() + 1) {
                return Err(Diagnostic::unsupported(
                    "list record-shape budget exhausted",
                    stmt.span,
                ));
            }
            if fields.iter().any(|(field, _, _)| field == &name.as_deref()) {
                return Ok(Fit::Unknown);
            }
            let names: Vec<_> = fields.iter().filter_map(|(name, _, _)| *name).collect();
            if self.list_shadowed(value, &names)? {
                return Ok(Fit::Unknown);
            }
            if name.is_none()
                && matches!(
                    value.kind,
                    ExprKind::Block(_) | ExprKind::Name(_) | ExprKind::Call { .. }
                )
            {
                return Ok(Fit::Unknown);
            }
            if *mutable && name.is_none() {
                return Ok(Fit::Unknown);
            }
            fields.push((name.as_deref(), value, *mutable));
        }
        let mut choices = Vec::new();
        for ty in expected.members() {
            let mut fit = Fit::Yes;
            if let Type::Record {
                primary,
                fields: members,
            } = ty
            {
                if !self
                    .flow
                    .spend(fields.len().saturating_mul(members.len()).saturating_mul(2))
                {
                    return Err(Diagnostic::unsupported(
                        "list record-shape budget exhausted",
                        block.span,
                    ));
                }
                for (name, value, mutable) in &fields {
                    let slot = if let Some(name) = name {
                        members.iter().find_map(|field| {
                            (&field.name == name).then_some((&field.ty, field.mutable))
                        })
                    } else {
                        Some((primary.as_ref(), false))
                    };
                    fit = fit.and(if let Some((slot, expected)) = slot {
                        if expected != *mutable {
                            Fit::No
                        } else {
                            self.list_probe(value, slot, reach)?
                        }
                    } else {
                        Fit::No
                    });
                }
                if !fields.iter().any(|(name, _, _)| name.is_none())
                    && !primary.accepts(&Type::Null)
                {
                    fit = Fit::No;
                }
                if members.iter().any(|member| {
                    !member.ty.accepts(&Type::Null)
                        && !fields
                            .iter()
                            .any(|(field, _, _)| *field == Some(member.name.as_str()))
                }) {
                    fit = Fit::No;
                }
            } else if fields.iter().any(|(name, _, _)| name.is_some()) {
                fit = Fit::No;
            } else if let Some((_, value, _)) = fields.first() {
                fit = self.list_probe(value, ty, reach)?;
            } else if !ty.accepts(&Type::Null) {
                fit = Fit::No;
            }
            if fit != Fit::No {
                choices.push(fit);
            }
        }
        Ok(match choices.as_slice() {
            [] => Fit::No,
            [fit] => *fit,
            _ => Fit::Unknown,
        })
    }
}
