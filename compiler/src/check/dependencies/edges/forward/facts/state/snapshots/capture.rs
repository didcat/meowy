use super::*;
use versions::Definition;

impl Checker {
    pub(in super::super::super) fn scalar_snapshots(
        &mut self,
        graph: &graph::Graph<'_>,
        orders: &schedule::Orders,
        entries: &Entries,
        observations: &observations::Observations<'_, '_>,
        span: Span,
        parts: usize,
    ) -> Result<Snapshots> {
        if !self.flow.spend(1) {
            return Err(budget(span));
        }
        if orders.len() != entries.len() {
            return Err(invalid(span));
        }
        let mut snapshots = self.scalar_entry_snapshots(graph, entries, span, parts)?;
        for (&owner, order) in orders {
            if order.len() > MAX_EDGES
                || !self
                    .flow
                    .spend(entries.len().checked_ilog2().unwrap_or(0) as usize + 1)
            {
                return Err(budget(span));
            }
            let (entry, _) = entries.get(&owner).ok_or_else(|| invalid(span))?;
            if order.first() != Some(entry) {
                return Err(invalid(span));
            }
            for (position, &port) in order.iter().enumerate() {
                if !self.flow.spend(3) {
                    return Err(budget(span));
                }
                let node = graph.nodes.get(&port).ok_or_else(|| invalid(span))?;
                if node.site.owner != owner {
                    return Err(invalid(span));
                }
                if position == 0 {
                    continue;
                }
                let before = snapshots.incoming(graph, port, &mut self.flow, span)?;
                let bottom = match before {
                    Value::Known(id) => snapshots.states[id].reach == FALSE,
                    Value::Missing => false,
                };
                let after = if bottom
                    || matches!(port, Port::Entry(_) | Port::Normal(_) | Port::BlockEntry(_))
                    || observations.sources.contains_key(&port)
                {
                    before
                } else {
                    Value::Missing
                };
                snapshots.record(
                    port,
                    Row {
                        owner,
                        before,
                        after,
                    },
                    &mut self.flow,
                    span,
                )?;
            }
        }
        if snapshots.rows.len() != graph.nodes.len() {
            return Err(invalid(span));
        }
        snapshots.definition_rows(observations.definitions, &mut self.flow, span)?;
        Ok(snapshots)
    }
}

impl Snapshots {
    pub(self) fn definition_rows(
        &self,
        definitions: &definitions::Definitions,
        flow: &mut Flow,
        span: Span,
    ) -> Result<()> {
        if definitions.declarations.len() > MAX_EDGES
            || definitions.writes.len() > MAX_EDGES
            || !flow.spend(definitions.declarations.len() + definitions.writes.len() + 1)
        {
            return Err(budget(span));
        }
        for location in definitions
            .declarations
            .values()
            .chain(definitions.writes.values())
        {
            if !flow.spend(3) {
                return Err(budget(span));
            }
            let row = self.rows.get(&location.port).ok_or_else(|| invalid(span))?;
            if row.owner != location.version.owner {
                return Err(invalid(span));
            }
            match location.version.definition {
                Definition::Parameter(_) => {
                    if !self.seeds.contains_key(&location.port) || location.normal.is_some() {
                        return Err(invalid(span));
                    }
                }
                Definition::Bind(point) | Definition::Write(point) => {
                    if location.port != Port::Operation(point) {
                        return Err(invalid(span));
                    }
                    if let Value::Known(id) = row.after
                        && (row.before != row.after || self.states[id].reach != FALSE)
                    {
                        return Err(invalid(span));
                    }
                    if let Some(normal) = location.normal {
                        if !flow.spend(1) {
                            return Err(budget(span));
                        }
                        if normal != Port::Normal(point)
                            || self
                                .rows
                                .get(&normal)
                                .is_none_or(|row| row.owner != location.version.owner)
                        {
                            return Err(invalid(span));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
