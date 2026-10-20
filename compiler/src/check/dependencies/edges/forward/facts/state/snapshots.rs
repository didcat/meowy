use super::*;

mod branches;
mod capture;
mod expressions;
mod guards;
mod inputs;
mod storage;

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum View<'a> {
    Missing,
    Known(&'a State),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Value {
    Missing,
    Known(usize),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Row {
    pub(self) owner: usize,
    pub(self) before: Value,
    pub(self) after: Value,
}

#[derive(Debug)]
pub(in super::super) struct Snapshots {
    pub(self) states: Vec<State>,
    pub(self) seeds: HashMap<Port, usize>,
    pub(self) rows: HashMap<Port, Row>,
    pub(self) expressions: HashMap<PointId, expressions::Fact>,
    pub(self) guards: HashMap<PointId, guards::Fact>,
    pub(self) parts: usize,
}

impl Checker {
    pub(in super::super) fn scalar_entry_snapshots(
        &mut self,
        graph: &graph::Graph<'_>,
        entries: &Entries,
        span: Span,
        parts: usize,
    ) -> Result<Snapshots> {
        if entries.len() > super::super::super::entries::MAX_ENTRIES
            || !self.flow.spend(entries.len() + 1)
        {
            return Err(budget(span));
        }
        let mut snapshots = Snapshots {
            states: Vec::new(),
            seeds: HashMap::new(),
            rows: HashMap::new(),
            expressions: HashMap::new(),
            guards: HashMap::new(),
            parts: parts.min(MAX_EDGES),
        };
        for (&owner, (port, state)) in entries {
            if !self.flow.spend(3) {
                return Err(budget(span));
            }
            let node = graph.nodes.get(port).ok_or_else(|| invalid(span))?;
            if *port != Port::BlockEntry(node.site.block)
                || node.site.owner != owner
                || state.owner() != owner
            {
                return Err(invalid(span));
            }
            snapshots.parts = snapshots.parts.checked_sub(1).ok_or_else(|| budget(span))?;
            let value = snapshots.copy(state, &mut self.flow, span)?;
            let Value::Known(id) = value else {
                return Err(invalid(span));
            };
            if snapshots.seeds.insert(*port, id).is_some() {
                return Err(invalid(span));
            }
            let value = snapshots.incoming(graph, *port, &mut self.flow, span)?;
            snapshots.record(
                *port,
                Row {
                    owner,
                    before: value,
                    after: value,
                },
                &mut self.flow,
                span,
            )?;
        }
        Ok(snapshots)
    }
}

impl Snapshots {
    pub(in super::super) fn room(&self) -> usize {
        self.parts
    }

    #[cfg(test)]
    pub(in super::super) fn seed(&self, port: Port) -> Option<&State> {
        self.seeds.get(&port).and_then(|id| self.states.get(*id))
    }

    #[cfg(test)]
    pub(in super::super) fn before(&self, port: Port) -> Option<View<'_>> {
        self.rows.get(&port).and_then(|row| self.view(row.before))
    }

    #[cfg(test)]
    pub(in super::super) fn after(&self, port: Port) -> Option<View<'_>> {
        self.rows.get(&port).and_then(|row| self.view(row.after))
    }

    #[cfg(test)]
    pub(self) fn view(&self, value: Value) -> Option<View<'_>> {
        match value {
            Value::Missing => Some(View::Missing),
            Value::Known(id) => self.states.get(id).map(View::Known),
        }
    }

    pub(self) fn copy(&mut self, state: &State, flow: &mut Flow, span: Span) -> Result<Value> {
        if self.states.len() >= MAX_EDGES || !flow.spend(1) {
            return Err(budget(span));
        }
        let mut state = state.snapshot(flow, span, self.room())?;
        state.used = state.weight() + state.cells.len().saturating_sub(1);
        self.keep(state, span)
    }

    pub(self) fn keep(&mut self, state: State, span: Span) -> Result<Value> {
        if self.states.len() >= MAX_EDGES {
            return Err(budget(span));
        }
        self.parts = self
            .parts
            .checked_sub(state.allocation())
            .ok_or_else(|| budget(span))?;
        let id = self.states.len();
        self.states.push(state);
        Ok(Value::Known(id))
    }

    pub(self) fn record(
        &mut self,
        port: Port,
        row: Row,
        flow: &mut Flow,
        span: Span,
    ) -> Result<()> {
        if self.rows.len() >= MAX_EDGES || !flow.spend(4) {
            return Err(budget(span));
        }
        if self.rows.contains_key(&port) {
            return Err(invalid(span));
        }
        for value in [row.before, row.after] {
            if let Value::Known(id) = value
                && self
                    .states
                    .get(id)
                    .is_none_or(|state| state.owner() != row.owner)
            {
                return Err(invalid(span));
            }
        }
        self.parts = self.parts.checked_sub(2).ok_or_else(|| budget(span))?;
        self.rows.insert(port, row);
        Ok(())
    }
}

pub(super) fn budget(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-snapshot budget exhausted", span)
}

pub(super) fn invalid(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-snapshot identity mismatch", span)
}

#[cfg(test)]
mod tests;
