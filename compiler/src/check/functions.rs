#[cfg(test)]
mod signatures;

use super::{Checker, Result, Scope, Spec, Value};
use crate::ast::{self, ExprKind, Span, StmtKind};
use crate::diagnostic::Diagnostic;
use crate::flow::TRUE;
use crate::hir::{self, Type};
use std::collections::BTreeMap;

impl Checker {
    pub(crate) fn declare_function(
        &mut self,
        name: &str,
        annotation: Option<&ast::TypeExpr>,
        params: &[ast::Param],
        body: &ast::Block,
        span: Span,
    ) -> Result<()> {
        let (args, result) =
            self.construction_root(Span::new(span.start, body.span.start), |checker| {
                let result = annotation
                    .map(|ty| checker.source_function(ty, true))
                    .transpose()?;
                let args: Vec<_> = params
                    .iter()
                    .map(|param| checker.source_function(&param.ty, true))
                    .collect::<Result<_>>()?;
                Ok((args, result))
            })?;
        let id = self.functions.len();
        self.functions.push(None);
        self.declare(
            name,
            Value::Function {
                id,
                params: args.clone(),
                result: result.clone(),
            },
            span,
        )?;
        let params: Vec<_> = params.iter().zip(args).collect();
        let result = self.function(id, name, &params, body, result, span)?;
        if let Some(Value::Function { result: target, .. }) =
            self.scopes.last_mut().expect("scope").values.get_mut(name)
        {
            *target = Some(result);
        }
        Ok(())
    }

    pub(crate) fn forward(&mut self, stmts: &[ast::Stmt], start: usize) -> Result<usize> {
        let mut index = start;
        let mut names = BTreeMap::new();
        while let Some(ast::Stmt {
            kind: StmtKind::Forward { name, ty },
            span,
        }) = stmts.get(index)
        {
            let Spec::Function { params, result } =
                self.construction_root(ty.span, |checker| checker.source_spec(ty, true))?
            else {
                return Err(Self::error(
                    "E221",
                    "forward declarations require a concrete function signature",
                    *span,
                ));
            };
            let id = self.functions.len();
            self.functions.push(None);
            self.declare(
                name,
                Value::Function {
                    id,
                    params: params.clone(),
                    result: Some(result.clone()),
                },
                *span,
            )?;
            names.insert(name.clone(), (id, params, result));
            index += 1;
        }
        let count = names.len();
        if self.documentation.is_some() {
            for stmt in &stmts[start..index] {
                self.doc_stage(stmt.span.start)?;
            }
        }
        for _ in 0..count {
            let stmt = stmts.get(index).ok_or_else(|| {
                Self::error(
                    "E221",
                    "forward function group is missing a definition",
                    stmts[start].span,
                )
            })?;
            let (name, ty, params, body) = match &stmt.kind {
                StmtKind::Bind {
                    name,
                    ty,
                    mutable: false,
                    value:
                        ast::Expr {
                            kind: ExprKind::Function { params, body },
                            ..
                        },
                } => (name, ty, params, body),
                _ => {
                    return Err(Self::error(
                        "E221",
                        "only the reserved function definitions may follow forward signatures",
                        stmt.span,
                    ));
                }
            };
            let (id, expected_params, expected_result) = names.remove(name).ok_or_else(|| {
                Self::error(
                    "E221",
                    format!("`{name}` is not a pending forward definition"),
                    stmt.span,
                )
            })?;
            let (actual_params, actual_result) =
                self.construction_root(Span::new(stmt.span.start, body.span.start), |checker| {
                    let result = ty
                        .as_ref()
                        .map(|ty| checker.source_type(ty, true))
                        .transpose()?
                        .unwrap_or(expected_result.clone());
                    let args = params
                        .iter()
                        .map(|param| checker.source_function(&param.ty, true))
                        .collect::<Result<Vec<_>>>()?;
                    Ok((args, result))
                })?;
            if expected_params != actual_params || actual_result != expected_result {
                return Err(Self::error(
                    "E221",
                    format!("definition of `{name}` does not match its reserved signature"),
                    stmt.span,
                ));
            }
            let params: Vec<_> = params.iter().zip(actual_params).collect();
            self.function(id, name, &params, body, Some(expected_result), stmt.span)
                .map_err(|error| {
                    if error.code == "B001" && error.message.contains("captur") {
                        Self::error(
                            "E221",
                            "forward functions cannot capture enclosing locals",
                            error.span,
                        )
                    } else {
                        error
                    }
                })?;
            index += 1;
        }
        Ok(index)
    }

    pub(crate) fn function(
        &mut self,
        id: usize,
        name: &str,
        params: &[(&ast::Param, Type)],
        body: &ast::Block,
        result: Option<Type>,
        span: Span,
    ) -> Result<Type> {
        let reach = std::mem::replace(&mut self.reach, TRUE);
        let owner = self.owner;
        self.owner = id + 1;
        if self.documentation.is_some() {
            self.scopes
                .last_mut()
                .expect("scope")
                .doc_values
                .insert(name.into(), span.start);
        }
        self.scopes.push(Scope::default());
        let mut ids = Vec::new();
        for (param, ty) in params {
            let id = self.local(ty.clone());
            self.places.insert(id);
            self.declare(
                &param.name,
                Value::Local {
                    id,
                    ty: ty.clone(),
                    mutable: false,
                    owner: self.owner,
                    constant: None,
                },
                param.span,
            )?;
            ids.push(id);
        }
        let block = self.block(body, result.clone(), None)?;
        let result = result.unwrap_or_else(|| block.ty.clone());
        self.functions[id] = Some(hir::Function {
            id,
            name: name.into(),
            params: ids,
            result: result.clone(),
            body: block,
        });
        self.doc_stage(span.start)?;
        self.scopes.pop();
        self.owner = owner;
        self.reach = reach;
        Ok(result)
    }

    pub(crate) fn call(
        &mut self,
        callee: &ast::Expr,
        args: &[ast::Expr],
        receiver: Option<&ast::Expr>,
        span: Span,
    ) -> Result<hir::Expr> {
        if receiver.is_none()
            && let ExprKind::Field { value, name } = &callee.kind
            && ["size", "add"].contains(&name.as_str())
            && self
                .symbol(value)?
                .is_none_or(|value| matches!(value, Value::Local { .. } | Value::Constant(_)))
        {
            return self.list_method(value, name, args, span);
        }
        let value = self.symbol(callee)?.ok_or_else(|| {
            Diagnostic::unsupported("indirect calls and callable fields", callee.span)
        })?;
        let args: Vec<_> = receiver.into_iter().chain(args.iter()).collect();
        let (kind, ty) = match value {
            Value::Foundation(item) => {
                return Err(Diagnostic::unsupported(
                    format!("runtime call to `{}`", item.name()),
                    callee.span,
                ));
            }
            Value::Print | Value::Panic => {
                if args.len() != 1 {
                    return Err(Self::error(
                        "E212",
                        "debug output operations take exactly one argument",
                        span,
                    ));
                }
                let mut parts = Vec::new();
                let points = self.format_parts(args[0], &mut parts)?;
                self.output_operation(
                    self.point.expect("output expression"),
                    matches!(value, Value::Panic),
                    &parts,
                    points,
                    span,
                )?;
                if matches!(value, Value::Print) {
                    (
                        hir::ExprKind::Print {
                            parts,
                            newline: true,
                        },
                        Type::Null,
                    )
                } else {
                    (hir::ExprKind::Panic { parts }, Type::Never)
                }
            }
            Value::Function { id, params, result } => {
                if params.len() != args.len() {
                    return Err(Self::error(
                        "E212",
                        format!(
                            "function expects {} arguments, found {}",
                            params.len(),
                            args.len()
                        ),
                        span,
                    ));
                }
                let result = result.ok_or_else(|| {
                    Diagnostic::unsupported(
                        "recursive functions without an explicit result annotation",
                        span,
                    )
                })?;
                let mutating = params.iter().any(|ty| matches!(ty, Type::Exclusive(_)));
                let mut values = Vec::new();
                let mut points = Vec::new();
                for (arg, ty) in args.into_iter().zip(params) {
                    let (point, value) = self.expr_point(arg, Some(&ty)).map_err(|error| {
                        if error.code == "E207" {
                            Self::error("E212", error.message, error.span)
                        } else {
                            error
                        }
                    })?;
                    points.push(point);
                    values.push(value);
                }
                if mutating {
                    self.forget_mutable();
                }
                let site = self.calls;
                self.calls += 1;
                self.proofs.calls.insert(site, self.reach);
                self.invocation(super::dependencies::Invocation {
                    point: self.point.expect("call expression"),
                    owner: self.owner,
                    function: id,
                    site,
                    args: points,
                    may_return: result != Type::Never,
                    control: self.control,
                    span,
                    edges: Vec::new(),
                })?;
                (
                    hir::ExprKind::Call {
                        id,
                        site,
                        args: values,
                    },
                    result,
                )
            }
            Value::Control { .. } => {
                return Err(Diagnostic::unsupported(
                    "scope control calls in value expressions",
                    span,
                ));
            }
            _ => return Err(Self::error("E212", "value is not callable", callee.span)),
        };
        Ok(hir::Expr { kind, ty, span })
    }
}
