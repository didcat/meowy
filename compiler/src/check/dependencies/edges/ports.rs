use super::{inventory::Family, *};

impl Checker {
    pub(crate) fn port_owner(&mut self, port: Port, span: Span) -> Result<usize> {
        let invalid = || Diagnostic::unsupported("proof graph-port identity mismatch", span);
        let cost = match port {
            Port::BlockEntry(_) | Port::BlockNormal(_) | Port::BlockResult(_) | Port::Leave(_) => {
                self.bodies.len().checked_ilog2().unwrap_or(0) as usize + 1
            }
            Port::Emission(_) => {
                self.emission_sources.len().checked_ilog2().unwrap_or(0) as usize
                    + self.emissions.len().checked_ilog2().unwrap_or(0) as usize
                    + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                    + 4
            }
            Port::Restart { .. } => {
                self.restart_inputs.len().checked_ilog2().unwrap_or(0) as usize
                    + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                    + 3
            }
            _ => 1,
        };
        if !self.flow.spend(cost) {
            return Err(Diagnostic::unsupported(
                "proof graph-port budget exhausted",
                span,
            ));
        }
        match port {
            Port::Entry(id)
            | Port::Normal(id)
            | Port::Operation(id)
            | Port::Snapshot(id)
            | Port::Prefix(id)
            | Port::Output { point: id, .. }
            | Port::Projection { point: id, .. }
            | Port::Conversion { point: id, .. }
            | Port::Address { point: id, .. }
            | Port::Reserve { point: id, .. } => {
                let owner = self
                    .points
                    .get(id)
                    .filter(|point| point.complete)
                    .map(|point| point.owner)
                    .ok_or_else(invalid)?;
                if !self.stage_port_valid(port, owner, span)? {
                    return Err(invalid());
                }
                Ok(owner)
            }
            Port::BlockEntry(id)
            | Port::BlockNormal(id)
            | Port::BlockResult(id)
            | Port::Leave(id) => self
                .bodies
                .get(&id)
                .map(|body| body.owner)
                .ok_or_else(invalid),
            Port::Emission(id) => {
                let (source, index) = self.emission_sources.get(&id).ok_or_else(invalid)?;
                let op = self.emissions.get(source).ok_or_else(invalid)?;
                let target = op.targets.get(*index).ok_or_else(invalid)?;
                if target.id != id
                    || !self
                        .points
                        .get(*source)
                        .is_some_and(|point| point.complete && point.owner == op.owner)
                    || !self
                        .bodies
                        .get(&target.block)
                        .is_some_and(|body| body.owner == op.owner)
                {
                    return Err(invalid());
                }
                Ok(op.owner)
            }
            Port::Restart { target, site } => {
                let input = self.restart_inputs.get(&site).ok_or_else(invalid)?;
                if input.target != target
                    || !input
                        .point
                        .and_then(|id| self.points.get(id))
                        .is_some_and(|point| {
                            point.complete
                                && point.kind == PointKind::Stmt
                                && point.owner == input.owner
                        })
                    || !self
                        .bodies
                        .get(&target)
                        .is_some_and(|body| body.owner == input.owner)
                {
                    return Err(invalid());
                }
                Ok(input.owner)
            }
        }
    }

    pub(crate) fn validate_edge_ports(
        &mut self,
        edges: &[(Family, Edge)],
        span: Span,
    ) -> Result<()> {
        if edges.len() > MAX_EDGES || !self.flow.spend(edges.len() + 1) {
            return Err(Diagnostic::unsupported(
                "proof graph-port budget exhausted",
                span,
            ));
        }
        for (_, edge) in edges {
            let from = self.port_owner(edge.from, span)?;
            let to = self.port_owner(edge.to, span)?;
            if from != to {
                return Err(Diagnostic::unsupported(
                    "proof graph-edge owner mismatch",
                    span,
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod owners;

mod selectors;
