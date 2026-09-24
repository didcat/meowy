use crate::ast::{self, ExprKind, StmtKind};
use crate::check::{Checker, Result};
use crate::diagnostic::Diagnostic;
use crate::hir::{self, Type};

use super::{Fit, MAX_SCALAR_NODES};

impl Checker {
    pub(crate) fn list_effect_block(
        &mut self,
        value: &ast::Expr,
        choices: &mut Vec<&Type>,
        unresolved: bool,
    ) -> Result<Option<hir::Expr>> {
        let Some((form, start)) = self.list_effect_form(value)? else {
            return Ok(None);
        };
        self.list_effect_body(form, start, choices, unresolved, value.span)
            .map(Some)
    }

    pub(crate) fn list_effect_form<'a>(
        &mut self,
        value: &'a ast::Expr,
    ) -> Result<Option<(&'a ast::Expr, usize)>> {
        let mut form = value;
        while let ExprKind::Group(value) = &form.kind {
            form = value;
        }
        let ExprKind::Block(block) = &form.kind else {
            return Ok(None);
        };
        if block.label.is_some() {
            return Ok(None);
        }
        let mut start = None;
        for (index, stmt) in block.stmts.iter().enumerate() {
            if !self.flow.spend(1) {
                return Err(Diagnostic::unsupported(
                    "effectful list block budget exhausted",
                    stmt.span,
                ));
            }
            match &stmt.kind {
                StmtKind::Emit {
                    label: None,
                    ty: None,
                    ..
                } => {
                    start.get_or_insert(index);
                }
                StmtKind::Bind { .. } | StmtKind::Assign { .. } | StmtKind::Expr(_)
                    if start.is_none() => {}
                _ => return Ok(None),
            }
        }
        Ok(start.filter(|start| *start > 0).map(|start| (form, start)))
    }

    pub(crate) fn list_effect_body(
        &mut self,
        form: &ast::Expr,
        start: usize,
        choices: &mut Vec<&Type>,
        unresolved: bool,
        span: ast::Span,
    ) -> Result<hir::Expr> {
        let ExprKind::Block(block) = &form.kind else {
            unreachable!()
        };
        let mut stmts = self.block_start(block, None, None, false)?;
        for stmt in &block.stmts[..start] {
            stmts.extend(self.stmt(stmt)?);
        }
        let mut collision = None;
        let mut nodes = 1usize;
        for stmt in &block.stmts[start..] {
            let StmtKind::Emit { value, .. } = &stmt.kind else {
                unreachable!()
            };
            let Some(scalar) = self.list_pure(value, true)? else {
                return Err(Diagnostic::unsupported(
                    "effectful list result needs unresolved lexical or effect context",
                    value.span,
                ));
            };
            nodes = nodes.saturating_add(scalar.nodes).saturating_add(1);
            if nodes > MAX_SCALAR_NODES {
                return Err(Diagnostic::unsupported(
                    "effectful list suffix budget exhausted",
                    stmt.span,
                ));
            }
            if let StmtKind::Emit {
                name: Some(name), ..
            } = &stmt.kind
                && self
                    .scopes
                    .last()
                    .expect("block scope")
                    .values
                    .contains_key(name)
            {
                collision = Some((name, stmt.span));
            }
        }
        if !self.flow.spend(
            block
                .span
                .end
                .saturating_sub(block.stmts[start].span.start)
                .saturating_add(1),
        ) {
            return Err(Diagnostic::unsupported(
                "effectful list suffix budget exhausted",
                block.span,
            ));
        }
        let suffix = ast::Expr {
            kind: ExprKind::Block(ast::Block {
                label: None,
                stmts: block.stmts[start..].to_vec(),
                span: block.span,
            }),
            span: form.span,
        };
        let mut matching = Vec::new();
        let mut unknown = false;
        let mut errors = Vec::new();
        for ty in choices.iter().copied() {
            let Type::List { element, .. } = ty else {
                unreachable!()
            };
            let Some(scalar) = self.list_pure(&suffix, true)? else {
                return Err(Diagnostic::unsupported(
                    "effectful list result needs unresolved lexical or effect context",
                    form.span,
                ));
            };
            let fit = match self.list_pure_probe(scalar, &suffix, element, self.reach) {
                Err(error) if ["E203", "E205", "E206"].contains(&error.code) => {
                    errors.push(error);
                    Fit::No
                }
                result => result?,
            };
            if fit != Fit::No {
                matching.push(ty);
                unknown |= fit == Fit::Unknown;
            }
        }
        if matching.is_empty() {
            if errors.len() == choices.len()
                && errors.iter().all(|error| error.code == errors[0].code)
            {
                return Err(errors.remove(0));
            }
            return Err(Self::error(
                "E207",
                "list element fits no expected list type",
                form.span,
            ));
        }
        if matching.len() > 1 {
            if let Some((name, span)) = collision {
                return Err(Self::error(
                    "E203",
                    format!("value `{name}` is already declared in this scope"),
                    span,
                ));
            }
            return Err(if unknown || unresolved {
                Diagnostic::unsupported(
                    "effectful list result has unresolved candidate constraints",
                    form.span,
                )
            } else {
                Self::error(
                    "E207",
                    "list literal fits multiple expected list types",
                    form.span,
                )
            });
        }
        let Type::List { element, .. } = matching[0] else {
            unreachable!()
        };
        self.frames.last_mut().expect("block frame").expected = Some(*element.clone());
        for stmt in &block.stmts[start..] {
            stmts.extend(self.stmt(stmt)?);
        }
        let block = self.block_end(block, stmts)?;
        let ty = block.ty.clone();
        *choices = matching;
        Ok(hir::Expr {
            kind: hir::ExprKind::Block(block),
            ty,
            span,
        })
    }
}

#[cfg(test)]
mod tests;
