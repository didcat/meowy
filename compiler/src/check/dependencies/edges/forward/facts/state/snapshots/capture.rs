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
        let absent = &observations.completion.absent;
        if absent.len() > MAX_EDGES || !self.flow.spend(absent.len() * 3 + 1) {
            return Err(budget(span));
        }
        for (&port, &owner) in absent {
            if !matches!(port, Port::Normal(_) | Port::Leave(_))
                || graph.nodes.contains_key(&port)
                || graph
                    .external
                    .get(&port)
                    .is_some_and(|site| site.owner != owner)
            {
                return Err(invalid(span));
            }
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
                if !self.flow.spend(2) {
                    return Err(budget(span));
                }
                let node = graph.nodes.get(&port).ok_or_else(|| invalid(span))?;
                if node.site.owner != owner {
                    return Err(invalid(span));
                }
                if position == 0 {
                    continue;
                }
                let before =
                    snapshots.incoming_with(graph, port, Some(absent), &mut self.flow, span)?;
                let bottom = match before {
                    Value::Known(id) => snapshots.states[id].reach == FALSE,
                    Value::Missing => false,
                };
                let after = if let Port::Emission(id) = port {
                    if let Some(input) = observations.emissions.get(&id) {
                        snapshots.emit(id, before, input, &mut self.flow, span)?
                    } else if bottom {
                        before
                    } else {
                        Value::Missing
                    }
                } else if before == Value::Missing
                    || bottom
                    || matches!(port, Port::Entry(_) | Port::Normal(_) | Port::BlockEntry(_))
                {
                    before
                } else if matches!(
                    port,
                    Port::Leave(_) | Port::BlockNormal(_) | Port::BlockResult(_)
                ) {
                    snapshots.complete(port, before, observations, &mut self.flow, span)?
                } else {
                    if !self.flow.spend(1) {
                        return Err(budget(span));
                    }
                    if observations.sources.contains_key(&port) {
                        before
                    } else {
                        if !self.flow.spend(1) {
                            return Err(budget(span));
                        }
                        if observations.forwarded.contains_key(&port) {
                            before
                        } else if let Port::Operation(point) = port {
                            if !self.flow.spend(
                                observations.calls.map_or(0, |calls| {
                                    calls.len().checked_ilog2().unwrap_or(0) as usize
                                }) + 1,
                            ) {
                                return Err(budget(span));
                            }
                            if let Some(call) =
                                observations.calls.and_then(|calls| calls.get(&point))
                            {
                                snapshots.call(
                                    before,
                                    call,
                                    observations.definitions,
                                    &mut self.flow,
                                    span,
                                )?
                            } else {
                                snapshots
                                    .storage(
                                        point,
                                        before,
                                        observations.definitions,
                                        &mut self.flow,
                                        span,
                                    )?
                                    .unwrap_or(Value::Missing)
                            }
                        } else {
                            Value::Missing
                        }
                    }
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
                if matches!(port, Port::Normal(_)) {
                    snapshots.expression(port, observations, &mut self.flow, span)?;
                    if !self.flow.spend(1) {
                        return Err(budget(span));
                    }
                    if let Some(input) = observations.guards.get(&port) {
                        snapshots.guard(port, input, &mut self.flow, span)?;
                    }
                }
            }
        }
        if snapshots.rows.len() != graph.nodes.len()
            || snapshots.guards.len() != observations.guards.len()
            || snapshots.emissions.len() != observations.emissions.len()
        {
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
                    if let Value::Known(id) = row.after {
                        let after = self.states.get(id).ok_or_else(|| invalid(span))?;
                        if after.owner() != row.owner {
                            return Err(invalid(span));
                        }
                        if after.reach == FALSE {
                            if row.before != row.after {
                                return Err(invalid(span));
                            }
                        } else {
                            if !flow.spend(after.cells.depth() + 2) {
                                return Err(budget(span));
                            }
                            if row.before == Value::Missing
                                || definitions
                                    .storage
                                    .get(&point)
                                    .is_none_or(|storage| storage.version != location.version)
                                || !matches!(after.get(location.version.local).map(|cell| &cell.data),
                                    Some(Data::Initialized { version: Some(version), .. })
                                        if *version == location.version)
                            {
                                return Err(invalid(span));
                            }
                        }
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
pub(super) mod tests;
