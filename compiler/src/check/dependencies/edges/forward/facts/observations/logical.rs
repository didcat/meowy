use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) struct Logical {
    pub(in super::super) condition: PointId,
    pub(in super::super) right: PointId,
    pub(in super::super) or: bool,
}

impl Checker {
    pub(super) fn scalar_logical_observations(
        &mut self,
        guards: &guards::Inputs,
        graph: &graph::Graph<'_>,
        span: Span,
        parts: usize,
    ) -> Result<(HashMap<Port, Logical>, usize)> {
        if guards.len() > MAX_EDGES || !self.flow.spend(guards.len() + 1) {
            return Err(budget(span));
        }
        let mut parts = parts.min(MAX_EDGES);
        let mut logical = HashMap::new();
        for (&port, input) in guards {
            if !self.flow.spend(2) {
                return Err(budget(span));
            }
            let point = self.points.get(input.point).ok_or_else(|| invalid(span))?;
            if point.kind == PointKind::Match {
                continue;
            }
            let Port::Normal(condition) = port else {
                return Err(invalid(span));
            };
            let or = match point.kind {
                PointKind::And => false,
                PointKind::Or => true,
                _ => return Err(invalid(span)),
            };
            let result = Port::Normal(input.point);
            if !graph.nodes.contains_key(&result) {
                continue;
            }
            if !self
                .flow
                .spend(self.region_edges.len().checked_ilog2().unwrap_or(0) as usize + 4)
            {
                return Err(budget(span));
            }
            let arm = input.arms[usize::from(or)];
            let edges = *self.region_edges.get(&arm).ok_or_else(|| invalid(span))?;
            let Port::Entry(right) = edges[0].to else {
                return Err(invalid(span));
            };
            let source = self.points.get(right).ok_or_else(|| invalid(span))?;
            if !source.complete
                || source.owner != input.source.owner
                || source.block != point.block
                || source.parent != Some(arm)
                || !matches!(
                    source.kind,
                    PointKind::Expr | PointKind::And | PointKind::Or
                )
                || edges
                    != [
                        Edge::new(Port::Entry(arm), Port::Entry(right), Route::Next),
                        Edge::new(Port::Normal(right), Port::Normal(arm), Route::Next),
                    ]
            {
                return Err(invalid(span));
            }
            let owner = input.source.owner;
            for (family, edge, count) in [
                (Family::Region, edges[0], 1),
                (Family::Region, edges[1], 1),
                (
                    Family::Branch,
                    Edge::new(Port::Normal(input.arms[0]), result, Route::Join),
                    2,
                ),
                (
                    Family::Branch,
                    Edge::new(Port::Normal(input.arms[1]), result, Route::Join),
                    2,
                ),
            ] {
                if !self.flow.spend(7) {
                    return Err(budget(span));
                }
                let Some(node) = graph.nodes.get(&edge.to) else {
                    continue;
                };
                let source = graph.nodes.get(&edge.from);
                let site = source
                    .map(|node| node.site)
                    .or_else(|| graph.external.get(&edge.from).copied());
                if node.site != self.scalar_port_site(edge.to, owner, span)?
                    || site != Some(self.scalar_port_site(edge.from, owner, span)?)
                    || node.incoming.len() != count
                    || node
                        .incoming
                        .iter()
                        .filter(|input| {
                            graph.edges.get(input.edge) == Some(&(family, edge))
                                && Some(input.source) == site
                                && input.reached == source.is_some()
                        })
                        .count()
                        != 1
                {
                    return Err(invalid(span));
                }
            }
            parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
            if logical
                .insert(
                    result,
                    Logical {
                        condition,
                        right,
                        or,
                    },
                )
                .is_some()
            {
                return Err(invalid(span));
            }
        }
        Ok((logical, parts))
    }
}

#[cfg(test)]
mod tests;
