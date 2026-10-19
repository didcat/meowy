use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Site {
    pub(super) owner: usize,
    pub(super) block: hir::BlockId,
    pub(super) span: Span,
}

pub(super) type Ports = BTreeMap<Port, Site>;

impl Checker {
    pub(super) fn scalar_ports(
        &mut self,
        reports: &Reports,
        entries: &state::Entries,
        span: Span,
        parts: usize,
    ) -> Result<(Ports, usize)> {
        if reports.entries.len() > super::super::entries::MAX_ENTRIES
            || !self.flow.spend(reports.entries.len() + 1)
        {
            return Err(budget(span));
        }
        if entries.len() != reports.entries.len() {
            return Err(invalid(span));
        }
        let mut parts = parts.min(MAX_EDGES);
        let mut ports = Ports::new();
        for (&owner, (block, walk)) in &reports.entries {
            if walk.ports.len() > parts
                || !self.flow.spend(
                    entries.len().checked_ilog2().unwrap_or(0) as usize + walk.ports.len() + 2,
                )
            {
                return Err(budget(span));
            }
            let (entry, state) = entries.get(&owner).ok_or_else(|| invalid(span))?;
            if *entry != Port::BlockEntry(*block)
                || state.owner() != owner
                || !walk.ports.contains(entry)
            {
                return Err(invalid(span));
            }
            for &port in &walk.ports {
                if !self
                    .flow
                    .spend(ports.len().checked_ilog2().unwrap_or(0) as usize + 1)
                {
                    return Err(budget(span));
                }
                parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
                let site = self.scalar_port_site(port, owner, span)?;
                if ports.insert(port, site).is_some() {
                    return Err(invalid(span));
                }
            }
        }
        Ok((ports, parts))
    }

    pub(super) fn scalar_port_site(
        &mut self,
        port: Port,
        owner: usize,
        span: Span,
    ) -> Result<Site> {
        if self.port_owner(port, span)? != owner {
            return Err(invalid(span));
        }
        let extra = match port {
            Port::Emission(_) => {
                self.emission_sources.len().checked_ilog2().unwrap_or(0) as usize
                    + self.emissions.len().checked_ilog2().unwrap_or(0) as usize
                    + 2
            }
            Port::Restart { .. } => {
                self.restart_inputs.len().checked_ilog2().unwrap_or(0) as usize + 1
            }
            _ => 1,
        };
        if !self
            .flow
            .spend(self.bodies.len().checked_ilog2().unwrap_or(0) as usize + extra + 2)
        {
            return Err(budget(span));
        }
        let (block, at) = match port {
            Port::BlockEntry(block)
            | Port::BlockNormal(block)
            | Port::BlockResult(block)
            | Port::Leave(block) => (block, None),
            Port::Emission(id) => {
                let (point, index) = self
                    .emission_sources
                    .get(&id)
                    .ok_or_else(|| invalid(span))?;
                let op = self.emissions.get(point).ok_or_else(|| invalid(span))?;
                let target = op.targets.get(*index).ok_or_else(|| invalid(span))?;
                (target.block, Some(op.span))
            }
            Port::Restart { target, site } => {
                let point = self
                    .restart_inputs
                    .get(&site)
                    .and_then(|input| input.point)
                    .ok_or_else(|| invalid(span))?;
                (
                    target,
                    Some(self.points.get(point).ok_or_else(|| invalid(span))?.span),
                )
            }
            Port::Entry(point)
            | Port::Normal(point)
            | Port::Operation(point)
            | Port::Snapshot(point)
            | Port::Prefix(point)
            | Port::Output { point, .. }
            | Port::Projection { point, .. }
            | Port::Conversion { point, .. }
            | Port::Address { point, .. }
            | Port::Reserve { point, .. } => {
                let point = self.points.get(point).ok_or_else(|| invalid(span))?;
                (point.block.ok_or_else(|| invalid(span))?, Some(point.span))
            }
        };
        let body = self.bodies.get(&block).ok_or_else(|| invalid(span))?;
        let at = at.unwrap_or(body.span);
        if body.owner != owner || at.start > at.end || body.span.start > body.span.end {
            return Err(invalid(span));
        }
        Ok(Site {
            owner,
            block,
            span: at,
        })
    }
}

pub(super) fn budget(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-port budget exhausted", span)
}

pub(super) fn invalid(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-port identity mismatch", span)
}

#[cfg(test)]
mod tests;
