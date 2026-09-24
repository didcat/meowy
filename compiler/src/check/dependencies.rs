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
            || self.derived_cells(id)
            || self.shaped(id).is_some_and(|shapes| {
                shapes.snapshots(&[]).any(|value| {
                    value
                        .origins
                        .roots
                        .iter()
                        .any(|root| self.derived_storage(*root))
                })
            })
            || self.record_pointees.get(&id).is_some_and(|fields| {
                fields
                    .values()
                    .any(|origins| origins.roots.iter().any(|root| self.derived_storage(*root)))
            })
            || self
                .pointees
                .get(&self.origin_id(id))
                .is_some_and(|origins| origins.roots.iter().any(|root| self.derived_storage(*root)))
    }

    pub(crate) fn derived_cells(&self, id: usize) -> bool {
        let mut pending = Vec::new();
        if let Some(cells) = self.reference_cells.get(&self.origin_id(id)) {
            pending.extend(&cells.places);
        }
        if let Some(fields) = self.record_cells.get(&id) {
            for cells in fields.values() {
                pending.extend(&cells.places);
            }
        }
        if let Some(shapes) = self.shaped(id) {
            for value in shapes.snapshots(&[]) {
                pending.extend(&value.cells.places);
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        while let Some((root, path)) = pending.pop() {
            if !seen.insert((root, path)) {
                continue;
            }
            if self.derived_storage(*root)
                || self.cell_origins(*root, path).is_some_and(|origins| {
                    origins.roots.iter().any(|root| self.derived_storage(*root))
                })
            {
                return true;
            }
            if let Some(cells) = self.stored_cells(*root, path) {
                pending.extend(&cells.places);
            }
            if self.record_pointees.get(root).is_some_and(|fields| {
                fields.iter().any(|(field, origins)| {
                    field.starts_with(path)
                        && origins.roots.iter().any(|root| self.derived_storage(*root))
                })
            }) {
                return true;
            }
            if let Some(shapes) = self.shaped(*root) {
                for value in shapes.snapshots(path) {
                    if value
                        .origins
                        .roots
                        .iter()
                        .any(|root| self.derived_storage(*root))
                    {
                        return true;
                    }
                    pending.extend(&value.cells.places);
                }
            }
            if let Some(fields) = self.record_cells.get(root) {
                for (field, cells) in fields {
                    if field.starts_with(path) {
                        pending.extend(&cells.places);
                    }
                }
            }
        }
        false
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
                    ExprKind::TemporaryBorrow { id, value, .. } => {
                        if self.derived_local(*id) {
                            return true;
                        }
                        pending.push(Node::Expr(value));
                    }
                    ExprKind::Unary { value, .. }
                    | ExprKind::Reborrow { value, .. }
                    | ExprKind::Coerce { value }
                    | ExprKind::TypeTest { value, .. }
                    | ExprKind::Deref(value)
                    | ExprKind::Primary(value)
                    | ExprKind::ListSize(value)
                    | ExprKind::StringSize(value) => pending.push(Node::Expr(value)),
                    ExprKind::Field { value, index } => {
                        if self
                            .field_origins(value, *index)
                            .roots
                            .iter()
                            .any(|root| self.derived_storage(*root))
                        {
                            return true;
                        }
                        pending.push(Node::Expr(value));
                    }
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
                        ..
                    } => {
                        pending.push(Node::Expr(condition));
                        pending.extend(then.iter().chain(otherwise).map(Node::Stmt));
                    }
                    Stmt::SlotAlias { id, .. } => {
                        if self.derived_local(*id) {
                            return true;
                        }
                    }
                    Stmt::Leave { .. } | Stmt::Restart { .. } => {}
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
pub(crate) use records::shapes::Shapes;
pub(crate) use references::{Cells, Origins};

#[cfg(test)]
mod stores;

#[cfg(test)]
mod retargets;

#[cfg(test)]
mod slots;

mod records;

#[cfg(test)]
mod indexed;

#[cfg(test)]
mod temporaries;

#[cfg(test)]
mod carriers;

#[cfg(test)]
mod cells;

#[cfg(test)]
mod cell_writes;

#[cfg(test)]
mod chains;

#[cfg(test)]
mod cell_slots;

mod calls;

#[cfg(test)]
mod record_locations;

mod exits;

mod restarts;
pub(crate) use restarts::RestartInput;

mod bodies;
pub(crate) use bodies::Body;

mod sites;
pub(crate) use sites::Site;

mod points;
pub(crate) use points::{Kind as PointKind, Point};

mod edges;
pub(crate) use edges::Edge;

mod sequences;
pub(crate) use sequences::{Sequence, Source as SequenceSource};

mod scope_exits;
pub(crate) use scope_exits::ScopeExit;

mod operations;
pub(crate) use operations::{Kind as OperationKind, Operation};

mod path_operations;
pub(crate) use path_operations::{Operation as PathOperation, Step as PathStep};

mod emissions;
pub(crate) use emissions::Emission;
