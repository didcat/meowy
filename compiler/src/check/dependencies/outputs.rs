use super::{
    PointKind,
    bodies::{Completion, completion::Shape},
    edges::{Edge, Port, Route},
};
use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Input {
    pub(crate) point: hir::PointId,
    pub(crate) primary: bool,
    pub(crate) source: Option<Shape>,
}

impl Input {
    pub(crate) fn valid_source(self, part: usize, stopped: Option<usize>) -> bool {
        if !self.primary {
            return self.source.is_none();
        }
        let Some(source) = self.source else {
            return false;
        };
        matches!(
            source,
            Shape::Scalar(_) | Shape::Union { .. } | Shape::Never
        ) && (Completion {
            normal: source != Shape::Never,
            result: source,
        })
        .valid()
            && if source == Shape::Never {
                stopped.is_some_and(|stop| stop <= part)
            } else {
                stopped != Some(part)
            }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Output {
    pub(crate) owner: usize,
    pub(crate) panic: bool,
    pub(crate) parts: Vec<Option<Input>>,
    pub(crate) stopped: Option<usize>,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn output_operation(
        &mut self,
        id: hir::PointId,
        panic: bool,
        parts: &[hir::Expr],
        points: Vec<Option<Input>>,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof output-operation budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof output-operation identity mismatch", span);
        if points.len() > super::sequences::MAX_ITEMS
            || !self.flow.spend(
                points.len() * (points.len().checked_ilog2().unwrap_or(0) as usize + 6)
                    + self.outputs.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + 4,
            )
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if point.kind != PointKind::Expr
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || points.len() != parts.len()
        {
            return Err(invalid());
        }
        let stopped = parts.iter().position(|part| part.ty == hir::Type::Never);
        let mut seen = std::collections::BTreeSet::new();
        for (index, (child, part)) in points.iter().zip(parts).enumerate() {
            if let Some(child) = child {
                if !child.valid_source(index, stopped)
                    || !seen.insert(child.point)
                    || !self.points.get(child.point).is_some_and(|child| {
                        child.parent == Some(id)
                            && child.owner == self.owner
                            && child.block == point.block
                            && child.complete
                            && matches!(
                                child.kind,
                                PointKind::Expr | PointKind::And | PointKind::Or
                            )
                    })
                {
                    return Err(invalid());
                }
                if child.primary
                    && (!matches!(&part.kind, hir::ExprKind::Primary(value)
                        if super::Coercion::primary_source(&value.ty) == child.source)
                        || child.source != Some(Completion::of(&part.ty).result))
                {
                    return Err(invalid());
                }
            } else if !matches!(part.kind, hir::ExprKind::String(_)) || part.ty != hir::Type::String
            {
                return Err(invalid());
            }
        }
        let mut edges = Vec::new();
        let (mut from, mut route) = (Port::Entry(id), Route::Next);
        if panic {
            edges.push(Edge::new(from, Port::Prefix(id), route));
            (from, route) = (Port::Prefix(id), Route::Returned);
        }
        for (part, child) in points.iter().enumerate() {
            if let Some(child) = child {
                edges.push(Edge::new(from, Port::Entry(child.point), route));
                (from, route) = (Port::Normal(child.point), Route::Next);
                if child.primary {
                    let stage = Port::Projection {
                        point: id,
                        step: part,
                    };
                    edges.push(Edge::new(from, stage, route));
                    (from, route) = (stage, Route::Next);
                }
            }
            if stopped == Some(part) {
                break;
            }
            let output = Port::Output { point: id, part };
            edges.push(Edge::new(from, output, route));
            (from, route) = (output, Route::Returned);
        }
        if stopped.is_none() {
            edges.push(Edge::new(from, Port::Operation(id), route));
            if !panic {
                edges.push(Edge::new(
                    Port::Operation(id),
                    Port::Normal(id),
                    Route::Returned,
                ));
            }
        }
        let output = Output {
            owner: self.owner,
            panic,
            parts: points,
            stopped,
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.outputs.get(&id) {
            return if *prior == output {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(output.edges.len()) {
            return Err(budget());
        }
        self.output_edges += output.edges.len();
        self.outputs.insert(id, output);
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod primary;

#[cfg(test)]
mod sources;
