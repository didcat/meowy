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
pub(crate) struct Borrow {
    pub(crate) owner: usize,
    pub(crate) place: hir::Place,
    pub(crate) storage: hir::LocalId,
    pub(crate) mode: hir::ReferenceMode,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn place_borrow_operation(
        &mut self,
        id: hir::PointId,
        value: &hir::Expr,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof place-borrow budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof place-borrow identity mismatch", span);
        let hir::ExprKind::Borrow(place) = &value.kind else {
            return Err(invalid());
        };
        let hir::Type::Reference(target) = &value.ty else {
            return Err(invalid());
        };
        if place.fields.len() > crate::list::MAX_WRITE_PATH
            || !self.flow.spend(
                place.fields.len() * 2
                    + self.place_borrows.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + self.places.len().checked_ilog2().unwrap_or(0) as usize
                    + 4,
            )
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        let storage = self
            .proofs
            .aliases
            .get(&place.root)
            .map_or(place.root, |alias| alias.root);
        if point.kind != PointKind::Expr
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || self.locals.get(storage).is_none()
            || (!self.places.contains(&place.root)
                && !self.proofs.aliases.contains_key(&place.root))
        {
            return Err(invalid());
        }
        let mut ty = self.locals.get(place.root).ok_or_else(invalid)?;
        for field in &place.fields {
            let hir::Type::Record { fields, .. } = ty else {
                return Err(invalid());
            };
            ty = &fields.get(*field).ok_or_else(invalid)?.ty;
        }
        let size = crate::borrow_contract::type_weight(ty, &mut self.flow, span)?;
        let count = crate::borrow_contract::type_weight(target, &mut self.flow, span)?;
        if !self.flow.spend(size.saturating_add(count)) {
            return Err(budget());
        }
        if ty != target.as_ref() {
            return Err(invalid());
        }
        let address = |step| Port::Address { point: id, step };
        let mut edges = vec![Edge::new(Port::Entry(id), address(0), Route::Next)];
        for step in 0..place.fields.len() {
            edges.push(Edge::new(address(step), address(step + 1), Route::Next));
        }
        edges.extend([
            Edge::new(
                address(place.fields.len()),
                Port::Operation(id),
                Route::Next,
            ),
            Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
        ]);
        let borrow = Borrow {
            owner: self.owner,
            place: place.clone(),
            storage,
            mode: hir::ReferenceMode::Shared,
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.place_borrows.get(&id) {
            return if *prior == borrow {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(borrow.edges.len()) {
            return Err(budget());
        }
        self.place_borrow_edges += borrow.edges.len();
        self.place_borrows.insert(id, borrow);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
