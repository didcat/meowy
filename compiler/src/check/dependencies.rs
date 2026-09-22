use super::Checker;
use crate::hir::{Expr, ExprKind, Stmt, WriteStep};

pub(crate) enum Node<'a> {
    Expr(&'a Expr),
    Stmt(&'a Stmt),
}

impl Checker {
    pub(crate) fn mark_derived(&mut self, id: usize) {
        self.derived.insert(id);
        if let Some(alias) = self.proofs.aliases.get(&id) {
            self.derived.insert(alias.root);
        }
    }

    pub(crate) fn derived_local(&self, id: usize) -> bool {
        self.derived_storage(id)
            || self
                .pointees
                .get(&self.origin_id(id))
                .is_some_and(|origins| origins.roots.iter().any(|root| self.derived_storage(*root)))
    }

    pub(crate) fn derived_storage(&self, id: usize) -> bool {
        self.derived.contains(&id)
            || self
                .proofs
                .aliases
                .get(&id)
                .is_some_and(|alias| self.derived.contains(&alias.root))
            || self.inputs.get(&id).is_some_and(|input| input.derived)
            || self.bool_inputs.get(&id).is_some_and(|input| input.derived)
            || self
                .record_inputs
                .get(&id)
                .is_some_and(|input| input.input.derived)
    }

    pub(crate) fn derived_expr(&self, expr: &Expr) -> bool {
        let mut pending = vec![Node::Expr(expr)];
        while let Some(node) = pending.pop() {
            match node {
                Node::Expr(expr) => match &expr.kind {
                    ExprKind::Local(id) => {
                        if self.derived_local(*id) {
                            return true;
                        }
                    }
                    ExprKind::Borrow(place) | ExprKind::ExclusivePath { place, .. } => {
                        if self.derived_local(place.root) {
                            return true;
                        }
                        if let ExprKind::ExclusivePath { path, .. } = &expr.kind {
                            for step in path {
                                if let WriteStep::Index(step) = step {
                                    pending.push(Node::Expr(&step.index));
                                }
                            }
                        }
                    }
                    ExprKind::Unary { value, .. }
                    | ExprKind::TemporaryBorrow { value, .. }
                    | ExprKind::Reborrow { value, .. }
                    | ExprKind::Field { value, .. }
                    | ExprKind::Coerce { value }
                    | ExprKind::TypeTest { value, .. }
                    | ExprKind::Deref(value)
                    | ExprKind::Primary(value)
                    | ExprKind::ListSize(value)
                    | ExprKind::StringSize(value) => pending.push(Node::Expr(value)),
                    ExprKind::Binary { left, right, .. } => {
                        pending.extend([Node::Expr(left), Node::Expr(right)]);
                    }
                    ExprKind::ListIndex { value, index }
                    | ExprKind::ElementBorrow { value, index, .. } => {
                        pending.extend([Node::Expr(value), Node::Expr(index)]);
                    }
                    ExprKind::ListAdd { value, item } => {
                        pending.extend([Node::Expr(value), Node::Expr(item)]);
                    }
                    ExprKind::Call { args: values, .. }
                    | ExprKind::List { values, .. }
                    | ExprKind::Print { parts: values, .. }
                    | ExprKind::Panic { parts: values } => {
                        pending.extend(values.iter().map(Node::Expr));
                    }
                    ExprKind::Block(block) => {
                        pending.extend(block.stmts.iter().map(Node::Stmt));
                    }
                    ExprKind::Null
                    | ExprKind::Bool(_)
                    | ExprKind::Int(_)
                    | ExprKind::Float(_)
                    | ExprKind::String(_)
                    | ExprKind::Heap => {}
                },
                Node::Stmt(stmt) => match stmt {
                    Stmt::Statement { stmts, .. } => {
                        pending.extend(stmts.iter().map(Node::Stmt));
                    }
                    Stmt::Bind { value, .. }
                    | Stmt::Assign { value, .. }
                    | Stmt::Emit { value, .. }
                    | Stmt::Expr(value) => {
                        pending.push(Node::Expr(value));
                    }
                    Stmt::SetPath { path, value, .. } => {
                        pending.push(Node::Expr(value));
                        for step in path {
                            if let WriteStep::Index(step) = step {
                                pending.push(Node::Expr(&step.index));
                            }
                        }
                    }
                    Stmt::Store { target, value, .. } => {
                        pending.extend([Node::Expr(target), Node::Expr(value)]);
                    }
                    Stmt::If {
                        condition,
                        then,
                        otherwise,
                    } => {
                        pending.push(Node::Expr(condition));
                        pending.extend(then.iter().chain(otherwise).map(Node::Stmt));
                    }
                    Stmt::SlotAlias { id, .. } => {
                        if self.derived_local(*id) {
                            return true;
                        }
                    }
                    Stmt::Leave(_) | Stmt::Restart { .. } => {}
                },
            }
        }
        false
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod control;

#[cfg(test)]
mod writes;

#[cfg(test)]
mod paths;

#[cfg(test)]
mod aliases;

mod references;
pub(crate) use references::Origins;

#[cfg(test)]
mod stores;

#[cfg(test)]
mod retargets;

#[cfg(test)]
mod slots;
