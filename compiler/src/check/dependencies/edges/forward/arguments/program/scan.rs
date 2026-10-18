use crate::ast::Span;
use crate::diagnostic::Diagnostic;
use crate::flow::Flow;
use crate::hir::{Expr, ExprKind, Stmt, WriteStep};

pub(super) enum Item<'a> {
    Statements(&'a [Stmt]),
    Statement(&'a Stmt),
    Expression(&'a Expr),
    Steps(&'a [WriteStep]),
}

pub(super) struct Scan<'a> {
    pub(super) pending: Vec<(Item<'a>, usize)>,
}

impl<'a> Scan<'a> {
    pub(super) fn next(
        &mut self,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Option<&'a Expr>, Diagnostic> {
        while let Some(expr) = self.next_expr(flow, span)? {
            if matches!(expr.kind, ExprKind::Call { .. }) {
                return Ok(Some(expr));
            }
        }
        Ok(None)
    }

    pub(super) fn next_expr(
        &mut self,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Option<&'a Expr>, Diagnostic> {
        while let Some((item, depth)) = self.pending.pop() {
            if !flow.spend(1) {
                return Err(Diagnostic::unsupported(
                    "proof HIR-call traversal budget exhausted",
                    span,
                ));
            }
            let mut add = |item| {
                if self.pending.len() >= 65_536 || depth >= 256 || !flow.spend(1) {
                    return Err(Diagnostic::unsupported(
                        "proof HIR-call traversal budget exhausted",
                        span,
                    ));
                }
                self.pending.push((item, depth + 1));
                Ok(())
            };
            match item {
                Item::Statements(stmts) => {
                    for stmt in stmts.iter().rev() {
                        add(Item::Statement(stmt))?;
                    }
                }
                Item::Steps(steps) => {
                    for step in steps.iter().rev() {
                        if let WriteStep::Index(step) = step {
                            add(Item::Expression(&step.index))?;
                        }
                    }
                }
                Item::Statement(stmt) => match stmt {
                    Stmt::Statement { stmts, .. } => add(Item::Statements(stmts))?,
                    Stmt::Bind { value, .. }
                    | Stmt::Assign { value, .. }
                    | Stmt::Emit { value, .. }
                    | Stmt::Expr(value) => add(Item::Expression(value))?,
                    Stmt::Store { target, value, .. } => {
                        add(Item::Expression(value))?;
                        add(Item::Expression(target))?;
                    }
                    Stmt::SetPath { path, value, .. } => {
                        add(Item::Expression(value))?;
                        add(Item::Steps(path))?;
                    }
                    Stmt::If {
                        condition,
                        then,
                        otherwise,
                        ..
                    } => {
                        add(Item::Statements(otherwise))?;
                        add(Item::Statements(then))?;
                        add(Item::Expression(condition))?;
                    }
                    Stmt::SlotAlias { .. } | Stmt::Leave { .. } | Stmt::Restart { .. } => {}
                },
                Item::Expression(expr) => {
                    match &expr.kind {
                        ExprKind::Block(block) => add(Item::Statements(&block.stmts))?,
                        ExprKind::Binary { left, right, .. } => {
                            add(Item::Expression(right))?;
                            add(Item::Expression(left))?;
                        }
                        ExprKind::TemporaryBorrow { value, .. }
                        | ExprKind::Reborrow { value, .. }
                        | ExprKind::Deref(value)
                        | ExprKind::Unary { value, .. }
                        | ExprKind::Field { value, .. }
                        | ExprKind::Primary(value)
                        | ExprKind::StringSize(value)
                        | ExprKind::ListSize(value)
                        | ExprKind::Coerce { value }
                        | ExprKind::TypeTest { value, .. } => add(Item::Expression(value))?,
                        ExprKind::ExclusivePath { path, .. } => add(Item::Steps(path))?,
                        ExprKind::ElementBorrow { value, index, .. }
                        | ExprKind::ListIndex { value, index }
                        | ExprKind::ListAdd { value, item: index } => {
                            add(Item::Expression(index))?;
                            add(Item::Expression(value))?;
                        }
                        ExprKind::List { values, .. }
                        | ExprKind::Call { args: values, .. }
                        | ExprKind::Print { parts: values, .. }
                        | ExprKind::Panic { parts: values } => {
                            for value in values.iter().rev() {
                                add(Item::Expression(value))?;
                            }
                        }
                        ExprKind::Null
                        | ExprKind::Bool(_)
                        | ExprKind::Int(_)
                        | ExprKind::Float(_)
                        | ExprKind::String(_)
                        | ExprKind::Heap
                        | ExprKind::Local(_)
                        | ExprKind::Borrow(_) => {}
                    }
                    return Ok(Some(expr));
                }
            }
        }
        Ok(None)
    }
}
