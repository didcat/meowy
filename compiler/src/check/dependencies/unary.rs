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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Stopped,
    Not,
    BitsNot,
    Negate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Unary {
    pub(crate) owner: usize,
    pub(crate) input: hir::PointId,
    pub(crate) kind: Kind,
    pub(crate) primary: bool,
    pub(crate) ty: hir::Type,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn unary_operation(
        &mut self,
        id: hir::PointId,
        input: hir::PointId,
        primary: bool,
        value: &hir::Expr,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof unary-operation budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof unary-operation identity mismatch", span);
        if !self
            .flow
            .spend(self.unaries.len().checked_ilog2().unwrap_or(0) as usize * 2 + 4)
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
        let kind = if value.ty == hir::Type::Never {
            if primary {
                return Err(invalid());
            }
            Kind::Stopped
        } else {
            let hir::ExprKind::Unary { op, value: child } = &value.kind else {
                return Err(invalid());
            };
            if child.ty != value.ty {
                return Err(invalid());
            }
            if primary
                && !matches!(&child.kind, hir::ExprKind::Primary(input) if matches!(input.ty, hir::Type::Record { .. }))
            {
                return Err(invalid());
            }
            match (op.as_str(), &value.ty) {
                ("!", hir::Type::Bool) => Kind::Not,
                ("~", hir::Type::Int { .. }) => Kind::BitsNot,
                ("-", hir::Type::Int { signed: true, .. } | hir::Type::Float { .. }) => {
                    Kind::Negate
                }
                _ => return Err(invalid()),
            }
        };
        let mut edges = vec![Edge::new(Port::Entry(id), Port::Entry(input), Route::Next)];
        if kind != Kind::Stopped {
            let mut from = Port::Normal(input);
            if primary {
                let stage = Port::Projection { point: id, step: 0 };
                edges.push(Edge::new(from, stage, Route::Next));
                from = stage;
            }
            let route = if kind == Kind::Negate && matches!(value.ty, hir::Type::Int { .. }) {
                Route::Checked
            } else {
                Route::Next
            };
            edges.extend([
                Edge::new(from, Port::Operation(id), Route::Next),
                Edge::new(Port::Operation(id), Port::Normal(id), route),
            ]);
        }
        let unary = Unary {
            owner: self.owner,
            input,
            kind,
            primary,
            ty: value.ty.clone(),
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.unaries.get(&id) {
            return if *prior == unary {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(unary.edges.len()) {
            return Err(budget());
        }
        self.unary_edges += unary.edges.len();
        self.unaries.insert(id, unary);
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod primary;
