use std::collections::BTreeSet;

use crate::ast::{self, ExprKind, StmtKind};
use crate::check::{Checker, Constant, Result, Value};
use crate::diagnostic::Diagnostic;
use crate::flow::{FALSE, Guard, TRUE};
use crate::foundation::{Item, Module};
use crate::hir::Type;

use super::{Fit, MAX_SCALAR_NODES, Scalar};

impl Checker {
    pub(crate) fn list_deferred(&mut self, value: &ast::Expr) -> Result<bool> {
        if !self.flow.spend(
            value
                .span
                .end
                .saturating_sub(value.span.start)
                .saturating_add(1),
        ) {
            return Err(Diagnostic::unsupported(
                "list deferral budget exhausted",
                value.span,
            ));
        }
        if self.scalar_literal(value) {
            return Ok(true);
        }
        match &value.kind {
            ExprKind::Group(value) => self.list_deferred(value),
            ExprKind::List(values) => {
                for value in values {
                    if !self.list_deferred(value)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            ExprKind::Block(block) if block.label.is_none() => {
                for stmt in &block.stmts {
                    let StmtKind::Emit {
                        label: None,
                        ty: None,
                        value,
                        ..
                    } = &stmt.kind
                    else {
                        return Ok(false);
                    };
                    if !self.list_deferred(value)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            _ => Ok(self.list_scalar(value)?.is_some()),
        }
    }

    pub(crate) fn list_scalar(&mut self, value: &ast::Expr) -> Result<Option<Scalar>> {
        self.list_pure(value, false)
    }

    pub(crate) fn list_pure(
        &mut self,
        value: &ast::Expr,
        aggregate: bool,
    ) -> Result<Option<Scalar>> {
        let mut scalar = Scalar {
            checker: Self::new(),
            nodes: 0,
            bytes: 0,
            unknown: false,
            controls: Vec::new(),
        };
        scalar.checker.scopes[0].values.clear();
        let mut pending = vec![value];
        let mut emitted = BTreeSet::new();
        let mut used = BTreeSet::new();
        while let Some(value) = pending.pop() {
            scalar.nodes += 1;
            if scalar.nodes > MAX_SCALAR_NODES
                || pending.len().saturating_add(2) > MAX_SCALAR_NODES
                || !self.flow.spend(1)
            {
                return Err(Diagnostic::unsupported(
                    "scalar list probe budget exhausted",
                    value.span,
                ));
            }
            let bytes = match &value.kind {
                ExprKind::Call { callee, .. } | ExprKind::Dispatch { callee, .. } => {
                    let Some((_, args)) = self.bits_arguments(value)? else {
                        return Ok(None);
                    };
                    pending.extend(args.into_iter().rev());
                    pending.push(callee);
                    0
                }
                ExprKind::Field { value: base, name }
                    if matches!(self.symbol(value)?, Some(Value::Foundation(Item::Bits(_)))) =>
                {
                    pending.push(base);
                    name.len()
                }
                ExprKind::Import(name) if Module::resolve(name) == Some(Module::Bits) => name.len(),
                ExprKind::List(values) if aggregate => {
                    for value in values {
                        if pending.len() == MAX_SCALAR_NODES || !self.flow.spend(1) {
                            return Err(Diagnostic::unsupported(
                                "pure list suffix budget exhausted",
                                value.span,
                            ));
                        }
                        pending.push(value);
                    }
                    0
                }
                ExprKind::Block(block) if aggregate && block.label.is_none() => {
                    let mut bytes = 0usize;
                    for stmt in &block.stmts {
                        let StmtKind::Emit {
                            label: None,
                            name,
                            ty: None,
                            value,
                            ..
                        } = &stmt.kind
                        else {
                            return Ok(None);
                        };
                        if pending.len() == MAX_SCALAR_NODES
                            || !self
                                .flow
                                .spend(name.as_ref().map_or(1, |name| name.len() + 1))
                        {
                            return Err(Diagnostic::unsupported(
                                "pure list suffix budget exhausted",
                                value.span,
                            ));
                        }
                        if let Some(name) = name {
                            emitted.insert(name.as_str());
                            bytes = bytes.saturating_add(name.len());
                        }
                        pending.push(value);
                    }
                    bytes
                }
                ExprKind::Int(text) | ExprKind::Float(text) => text.len(),
                ExprKind::String(parts) => {
                    let mut bytes = parts.len();
                    for part in parts {
                        let ast::StringPart::Text(text) = part else {
                            return Ok(None);
                        };
                        bytes = bytes.saturating_add(text.len());
                    }
                    bytes
                }
                ExprKind::Group(value) => {
                    pending.push(value);
                    0
                }
                ExprKind::Unary { op, value } if ["-", "!", "~"].contains(&op.as_str()) => {
                    pending.push(value);
                    0
                }
                ExprKind::Binary { op, left, right }
                    if [
                        "+", "-", "*", "/", "%", "&", "|", "^", "&&", "||", "==", "!=", "<", ">",
                        "<=", ">=",
                    ]
                    .contains(&op.as_str()) =>
                {
                    if aggregate && ["&&", "||"].contains(&op.as_str()) {
                        let mut condition = left.as_ref();
                        while let ExprKind::Group(value) = &condition.kind {
                            if !self.flow.spend(1) {
                                return Err(Diagnostic::unsupported(
                                    "suffix control probe budget exhausted",
                                    condition.span,
                                ));
                            }
                            condition = value;
                        }
                        scalar.controls.push((condition.span, right.span));
                    }
                    pending.push(right);
                    pending.push(left);
                    0
                }
                ExprKind::Name(name) => {
                    if aggregate {
                        if !self.flow.spend(name.len() + 1) {
                            return Err(Diagnostic::unsupported(
                                "pure list suffix lookup budget exhausted",
                                value.span,
                            ));
                        }
                        used.insert(name.as_str());
                    }
                    if scalar.checker.scopes[0].values.contains_key(name) {
                        continue;
                    }
                    let lookups = self.scopes.iter().fold(0usize, |work, scope| {
                        work.saturating_add(
                            scope.values.len().checked_ilog2().unwrap_or(0) as usize + 1,
                        )
                    });
                    if !self.flow.spend(lookups.saturating_mul(name.len() + 1)) {
                        return Err(Diagnostic::unsupported(
                            "scalar list lookup budget exhausted",
                            value.span,
                        ));
                    }
                    let Some(symbol) = Self::list_symbol(&self.scopes, name) else {
                        return Ok(None);
                    };
                    let bits = matches!(
                        symbol,
                        Value::Module(Module::Bits) | Value::Foundation(Item::Bits(_))
                    );
                    let constant = match symbol {
                        _ if bits => None,
                        Value::Constant(value) => Some(value),
                        Value::Local {
                            ty,
                            mutable,
                            owner,
                            constant,
                            ..
                        } if *owner == self.owner
                            && (aggregate || !mutable && constant.is_some())
                            && matches!(
                                ty,
                                Type::Null
                                    | Type::Bool
                                    | Type::Int { .. }
                                    | Type::Float { .. }
                                    | Type::String
                            ) =>
                        {
                            if *mutable {
                                None
                            } else {
                                constant.as_ref()
                            }
                        }
                        _ => return Ok(None),
                    };
                    let bytes = name.len().saturating_add(match constant {
                        Some(Constant::String(value)) => value.len(),
                        _ => 0,
                    });
                    if !self.flow.spend(bytes) {
                        return Err(Diagnostic::unsupported(
                            "scalar list constant budget exhausted",
                            value.span,
                        ));
                    }
                    let symbol = match symbol {
                        _ if bits => symbol.clone(),
                        Value::Local { ty, .. } => {
                            let id = scalar.checker.locals.len();
                            scalar.checker.locals.push(ty.clone());
                            Value::Local {
                                id,
                                ty: ty.clone(),
                                mutable: false,
                                owner: 0,
                                constant: constant.cloned(),
                            }
                        }
                        _ => Value::Constant(constant.expect("compiler constant").clone()),
                    };
                    scalar.unknown |= constant.is_none() && !bits;
                    scalar.checker.scopes[0].values.insert(name.clone(), symbol);
                    bytes
                }
                _ => return Ok(None),
            };
            if pending.len() > MAX_SCALAR_NODES || !self.flow.spend(bytes) {
                return Err(Diagnostic::unsupported(
                    "scalar list probe budget exhausted",
                    value.span,
                ));
            }
            scalar.bytes = scalar.bytes.saturating_add(bytes);
        }
        if used.iter().any(|name| emitted.contains(name)) {
            return Ok(None);
        }
        Ok(Some(scalar))
    }

    pub(crate) fn list_scalar_probe(
        &mut self,
        value: &ast::Expr,
        expected: &Type,
        reach: Guard,
    ) -> Result<Option<Fit>> {
        let Some(scalar) = self.list_scalar(value)? else {
            return Ok(None);
        };
        self.list_pure_probe(scalar, value, expected, reach)
            .map(Some)
    }

    pub(crate) fn list_pure_probe(
        &mut self,
        mut scalar: Scalar,
        value: &ast::Expr,
        expected: &Type,
        reach: Guard,
    ) -> Result<Fit> {
        let weight = crate::borrow_contract::type_weight(expected, &mut self.flow, value.span)?;
        let names = scalar.checker.scopes[0].values.len();
        let work = scalar.nodes.saturating_mul(scalar.nodes).saturating_mul(
            names
                .saturating_add(weight)
                .saturating_add(scalar.bytes)
                .saturating_add(scalar.nodes)
                .saturating_add(1),
        );
        if !self.flow.spend(work) {
            return Err(Diagnostic::unsupported(
                "scalar list checking budget exhausted",
                value.span,
            ));
        }
        scalar.checker.reach = if reach == FALSE { FALSE } else { TRUE };
        let result = scalar.checker.expr(value, Some(expected));
        if scalar.checker.flow.exceeded() || !self.flow.spend(scalar.checker.flow.work) {
            return Err(Diagnostic::unsupported(
                "scalar list checking budget exhausted",
                value.span,
            ));
        }
        if scalar.unknown
            && reach != FALSE
            && let Err(error) = &result
        {
            if !self.flow.spend(scalar.controls.len() + 1) {
                return Err(Diagnostic::unsupported(
                    "suffix control probe budget exhausted",
                    value.span,
                ));
            }
            let uncertain = scalar.controls.iter().any(|(condition, right)| {
                right.start <= error.span.start
                    && error.span.end <= right.end
                    && scalar
                        .checker
                        .guards
                        .get(&(condition.start, condition.end))
                        .is_some_and(|guard| *guard > TRUE)
            });
            if uncertain && error.code != "B001" {
                let retry = self.list_pure(value, true)?.expect("validated pure suffix");
                match self.list_pure_probe(retry, value, expected, FALSE) {
                    Ok(Fit::Yes | Fit::Unknown) => return Ok(Fit::Unknown),
                    Err(error) if error.code == "B001" => return Err(error),
                    _ => {}
                }
            }
        }
        Ok(match result {
            Ok(_) => Fit::Yes,
            Err(error) if ["B001", "E203", "E205", "E206"].contains(&error.code) => {
                return Err(error);
            }
            Err(error)
                if error.code == "E207"
                    && (error.message.contains("multiple possible expected types")
                        || error.message.contains("fits multiple expected list types")) =>
            {
                Fit::Unknown
            }
            Err(_) => Fit::No,
        })
    }
}
