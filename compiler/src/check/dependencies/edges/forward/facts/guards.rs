use super::*;
use crate::check::dependencies::Branch;

mod controls;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Input {
    pub(super) point: PointId,
    pub(super) arms: [PointId; 2],
    pub(super) source: Branch,
    pub(super) control: Option<bool>,
}

pub(super) type Inputs = HashMap<Port, Input>;

impl Checker {
    pub(super) fn scalar_guard_inputs(
        &mut self,
        graph: &graph::Graph<'_>,
        span: Span,
        parts: usize,
    ) -> Result<(Inputs, usize)> {
        if self.branch_inputs.len() > MAX_EDGES
            || self.branch_edges.len() > MAX_EDGES / 6
            || !self.flow.spend(self.branch_edges.len() + 1)
        {
            return Err(budget(span));
        }
        let mut parts = parts.min(MAX_EDGES);
        let mut inputs = Inputs::new();
        let mut count = 0;
        for (&id, edges) in &self.branch_edges {
            let Some(&branch) = self.branch_inputs.get(&id) else {
                continue;
            };
            if !self
                .flow
                .spend(self.region_edges.len().checked_ilog2().unwrap_or(0) as usize + 20)
            {
                return Err(budget(span));
            }
            count += 1;
            let point = self.points.get(id).ok_or_else(|| invalid(span))?;
            let (Port::Entry(condition), Port::Entry(taken), Port::Entry(skipped)) =
                (edges[0].to, edges[1].to, edges[2].to)
            else {
                return Err(invalid(span));
            };
            if !point.complete
                || point.owner != branch.owner
                || !matches!(
                    point.kind,
                    PointKind::Match | PointKind::And | PointKind::Or
                )
            {
                return Err(invalid(span));
            }
            for (child, kind) in [
                (condition, PointKind::Condition),
                (taken, PointKind::Then),
                (skipped, PointKind::Else),
            ] {
                if !self.points.get(child).is_some_and(|child| {
                    child.complete
                        && child.kind == kind
                        && child.parent == Some(id)
                        && child.owner == branch.owner
                        && child.block == point.block
                }) {
                    return Err(invalid(span));
                }
            }
            let bypass = if point.kind == PointKind::Or {
                taken
            } else {
                skipped
            };
            if *edges
                != [
                    Edge::new(Port::Entry(id), Port::Entry(condition), Route::Next),
                    Edge::new(Port::Normal(condition), Port::Entry(taken), Route::True),
                    Edge::new(Port::Normal(condition), Port::Entry(skipped), Route::False),
                    Edge::new(Port::Normal(taken), Port::Normal(id), Route::Join),
                    Edge::new(Port::Normal(skipped), Port::Normal(id), Route::Join),
                    Edge::new(Port::Entry(bypass), Port::Normal(bypass), Route::Next),
                ]
            {
                return Err(invalid(span));
            }
            let source = self.points.get(branch.input).ok_or_else(|| invalid(span))?;
            let link = Edge::new(
                Port::Normal(branch.input),
                Port::Normal(condition),
                Route::Next,
            );
            if !source.complete
                || source.owner != branch.owner
                || source.block != point.block
                || source.parent != Some(condition)
                || !matches!(
                    source.kind,
                    PointKind::Expr | PointKind::And | PointKind::Or
                )
                || self.region_edges.get(&condition)
                    != Some(&[
                        Edge::new(
                            Port::Entry(condition),
                            Port::Entry(branch.input),
                            Route::Next,
                        ),
                        link,
                    ])
            {
                return Err(invalid(span));
            }
            let Some(node) = graph.nodes.get(&link.to) else {
                continue;
            };
            if !self.flow.spend(10) {
                return Err(budget(span));
            }
            let [input] = node.incoming.as_slice() else {
                return Err(invalid(span));
            };
            let from = graph.nodes.get(&link.from);
            let site = from
                .map(|node| node.site)
                .or_else(|| graph.external.get(&link.from).copied());
            if !branch.normal
                || node.site.owner != branch.owner
                || Some(node.site.block) != point.block
                || node.site.span != self.points[condition].span
                || input.source.owner != branch.owner
                || Some(input.source.block) != source.block
                || input.source.span != source.span
                || site != Some(input.source)
                || input.reached != from.is_some()
                || graph.edges.get(input.edge) != Some(&(Family::Region, link))
                || node.outgoing.len() != 2
                || !node
                    .outgoing
                    .iter()
                    .zip(&edges[1..3])
                    .all(|(&index, edge)| graph.edges.get(index) == Some(&(Family::Branch, *edge)))
            {
                return Err(invalid(span));
            }
            parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
            let control = controls::dependency(
                id,
                branch,
                &self.points,
                &self.branch_inputs,
                &mut self.flow,
                span,
            )?;
            if inputs
                .insert(
                    link.to,
                    Input {
                        point: id,
                        arms: [taken, skipped],
                        source: branch,
                        control,
                    },
                )
                .is_some()
            {
                return Err(invalid(span));
            }
        }
        if count != self.branch_inputs.len() {
            return Err(invalid(span));
        }
        Ok((inputs, parts))
    }
}

pub(super) fn budget(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-guard budget exhausted", span)
}

pub(super) fn invalid(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-guard identity mismatch", span)
}

#[cfg(test)]
mod tests;
