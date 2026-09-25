use super::{
    PointKind,
    edges::{Edge, Port, Route},
};
use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Field {
    pub(crate) owner: usize,
    pub(crate) input: hir::PointId,
    pub(crate) index: usize,
    pub(crate) load: bool,
    pub(crate) normal: bool,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn field_operation(
        &mut self,
        id: hir::PointId,
        input: hir::PointId,
        load: bool,
        value: &hir::Expr,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof field-operation budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof field-operation identity mismatch", span);
        if !self
            .flow
            .spend(self.fields.len().checked_ilog2().unwrap_or(0) as usize * 2 + 4)
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if point.kind != PointKind::Expr
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || !self.points.get(input).is_some_and(|child| {
                child.parent == Some(id)
                    && child.owner == self.owner
                    && child.block == point.block
                    && child.complete
                    && matches!(child.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            })
        {
            return Err(invalid());
        }
        let hir::ExprKind::Field {
            value: receiver,
            index,
        } = &value.kind
        else {
            return Err(invalid());
        };
        let hir::Type::Record { fields, .. } = &receiver.ty else {
            return Err(invalid());
        };
        let selected = fields.get(*index).ok_or_else(invalid)?;
        let size = crate::borrow_contract::type_weight(&selected.ty, &mut self.flow, span)?;
        let count = crate::borrow_contract::type_weight(&value.ty, &mut self.flow, span)?;
        if !self.flow.spend(size.saturating_add(count)) {
            return Err(budget());
        }
        if selected.ty != value.ty {
            return Err(invalid());
        }
        if load {
            let hir::ExprKind::Deref(pointer) = &receiver.kind else {
                return Err(invalid());
            };
            let hir::Type::Reference(ty) = &pointer.ty else {
                return Err(invalid());
            };
            let size = crate::borrow_contract::type_weight(ty, &mut self.flow, span)?;
            let count = crate::borrow_contract::type_weight(&receiver.ty, &mut self.flow, span)?;
            if !self.flow.spend(size.saturating_add(count)) {
                return Err(budget());
            }
            if ty.as_ref() != &receiver.ty {
                return Err(invalid());
            }
        }
        let normal = value.ty != hir::Type::Never;
        let mut edges = vec![Edge::new(Port::Entry(id), Port::Entry(input), Route::Next)];
        let mut from = Port::Normal(input);
        if load {
            let stage = Port::Projection { point: id, step: 0 };
            edges.push(Edge::new(from, stage, Route::Next));
            from = stage;
        }
        edges.push(Edge::new(from, Port::Operation(id), Route::Next));
        if normal {
            edges.push(Edge::new(
                Port::Operation(id),
                Port::Normal(id),
                Route::Next,
            ));
        }
        let field = Field {
            owner: self.owner,
            input,
            index: *index,
            load,
            normal,
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.fields.get(&id) {
            return if *prior == field {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(field.edges.len()) {
            return Err(budget());
        }
        self.field_edges += field.edges.len();
        self.fields.insert(id, field);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
