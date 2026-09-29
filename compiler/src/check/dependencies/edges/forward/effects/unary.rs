use super::*;
use crate::{
    check::dependencies::{ScalarKind, UnaryKind},
    hir::Type,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Projection,
    Operation,
    Result,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stage {
    pub(super) point: PointId,
    pub(super) owner: usize,
    pub(super) input: PointId,
    pub(super) op: UnaryKind,
    pub(super) ty: ScalarKind,
    pub(super) primary: bool,
    pub(super) checked: bool,
    pub(super) control: bool,
    pub(super) kind: Kind,
}

impl Checker {
    pub(super) fn unary_effect_stage(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<Option<Stage>> {
        let budget = || Diagnostic::unsupported("proof unary-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof unary-effect identity mismatch", span);
        let id = match port {
            Port::Projection { point, .. } | Port::Operation(point) | Port::Normal(point) => point,
            _ => return Ok(None),
        };
        if !self
            .flow
            .spend(self.unaries.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some(op) = self.unaries.get(&id) else {
            return Ok(None);
        };
        if !self.flow.spend(14) {
            return Err(budget());
        }
        let ty = match &op.ty {
            Type::Bool if op.kind == UnaryKind::Not => ScalarKind::Bool,
            Type::Int { bits, signed }
                if matches!(bits, 8 | 16 | 32 | 64)
                    && (op.kind == UnaryKind::BitsNot
                        || (op.kind == UnaryKind::Negate && *signed)) =>
            {
                ScalarKind::Int {
                    bits: *bits,
                    signed: *signed,
                }
            }
            Type::Float { bits } if matches!(bits, 32 | 64) && op.kind == UnaryKind::Negate => {
                ScalarKind::Float { bits: *bits }
            }
            _ => return Err(invalid()),
        };
        let point = self.points.get(id).ok_or_else(invalid)?;
        if op.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != op.span
            || !self.points.get(op.input).is_some_and(|input| {
                input.complete
                    && input.owner == owner
                    && input.parent == Some(id)
                    && input.block == point.block
                    && matches!(input.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            })
        {
            return Err(invalid());
        }
        let checked = op.kind == UnaryKind::Negate && matches!(ty, ScalarKind::Int { .. });
        let entry = Edge::new(Port::Entry(id), Port::Entry(op.input), Route::Next);
        let result = Edge::new(
            Port::Operation(id),
            Port::Normal(id),
            if checked { Route::Checked } else { Route::Next },
        );
        let projection = Port::Projection { point: id, step: 0 };
        let direct = [
            entry,
            Edge::new(Port::Normal(op.input), Port::Operation(id), Route::Next),
            result,
        ];
        let primary = [
            entry,
            Edge::new(Port::Normal(op.input), projection, Route::Next),
            Edge::new(projection, Port::Operation(id), Route::Next),
            result,
        ];
        let edges: &[Edge] = if op.primary { &primary } else { &direct };
        if op.edges.as_slice() != edges {
            return Err(invalid());
        }
        let kind = match port {
            Port::Projection { step: 0, .. } if op.primary => Kind::Projection,
            Port::Operation(_) => Kind::Operation,
            Port::Normal(_) => Kind::Result,
            _ => return Err(invalid()),
        };
        if kind != Kind::Projection {
            if !self
                .flow
                .spend(reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize + 1)
            {
                return Err(budget());
            }
            if reports.index.operations.get(&id) != Some(&owner) {
                return Err(invalid());
            }
        }
        Ok(Some(Stage {
            point: id,
            owner,
            input: op.input,
            op: op.kind,
            ty,
            primary: op.primary,
            checked,
            control: op.control,
            kind,
        }))
    }
}

#[cfg(test)]
mod tests;
