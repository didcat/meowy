use super::TreeNode;
use crate::ast::{Expr, ExprKind, StmtKind, StringPart};

pub(crate) fn bounded_tree(expr: &Expr) -> bool {
    let mut pending = vec![(TreeNode::Expr(expr), 1usize)];
    while let Some((node, depth)) = pending.pop() {
        if depth > 256 {
            return false;
        }
        let depth = depth + 1;
        match node {
            TreeNode::Stmt(stmt) => match &stmt.kind {
                StmtKind::Bind { value, .. }
                | StmtKind::Emit { value, .. }
                | StmtKind::Expr(value) => pending.push((TreeNode::Expr(value), depth)),
                StmtKind::Assign { target, value } => {
                    pending.push((TreeNode::Expr(target), depth));
                    pending.push((TreeNode::Expr(value), depth));
                }
                StmtKind::Match { arms } => {
                    for (condition, body) in arms {
                        if let Some(condition) = condition {
                            pending.push((TreeNode::Expr(condition), depth));
                        }
                        pending.push((TreeNode::Stmt(body), depth));
                    }
                }
                StmtKind::TypeAlias { .. } | StmtKind::Forward { .. } => {}
            },
            TreeNode::Expr(expr) => match &expr.kind {
                ExprKind::Unary { value, .. }
                | ExprKind::Group(value)
                | ExprKind::Field { value, .. }
                | ExprKind::Ascribe { value, .. }
                | ExprKind::Specialize { value, .. }
                | ExprKind::TypeQuery(value) => pending.push((TreeNode::Expr(value), depth)),
                ExprKind::Binary { left, right, .. } => {
                    pending.push((TreeNode::Expr(left), depth));
                    pending.push((TreeNode::Expr(right), depth));
                }
                ExprKind::Index { value, index } => {
                    pending.push((TreeNode::Expr(value), depth));
                    pending.push((TreeNode::Expr(index), depth));
                }
                ExprKind::Call { callee, args } | ExprKind::Dispatch { callee, args, .. } => {
                    pending.push((TreeNode::Expr(callee), depth));
                    for arg in args {
                        pending.push((TreeNode::Expr(arg), depth));
                    }
                    if let ExprKind::Dispatch { value, .. } = &expr.kind {
                        pending.push((TreeNode::Expr(value), depth));
                    }
                }
                ExprKind::Block(block)
                | ExprKind::Function { body: block, .. }
                | ExprKind::DispatchBlock { block, .. } => {
                    for stmt in &block.stmts {
                        pending.push((TreeNode::Stmt(stmt), depth));
                    }
                    if let ExprKind::DispatchBlock { value, .. } = &expr.kind {
                        pending.push((TreeNode::Expr(value), depth));
                    }
                }
                ExprKind::List(values) => {
                    for value in values {
                        pending.push((TreeNode::Expr(value), depth));
                    }
                }
                ExprKind::String(parts) => {
                    for part in parts {
                        if let StringPart::Value(value) = part {
                            pending.push((TreeNode::Expr(value), depth));
                        }
                    }
                }
                _ => {}
            },
        }
    }
    true
}
