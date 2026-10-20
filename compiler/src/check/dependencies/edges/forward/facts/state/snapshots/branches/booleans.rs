use super::*;
use domain::Values;

impl Snapshots {
    pub(super) fn boolean(
        &mut self,
        input: PointId,
        id: usize,
        value: bool,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Value> {
        if !flow.spend(6) {
            return Err(budget(span));
        }
        let before = Value::Known(id);
        let state = self.states.get(id).ok_or_else(|| invalid(span))?;
        let row = self
            .rows
            .get(&Port::Normal(input))
            .ok_or_else(|| invalid(span))?;
        let fact = self.expressions.get(&input).ok_or_else(|| invalid(span))?;
        let scalar = fact.value.as_ref().ok_or_else(|| invalid(span))?;
        let full = Domain {
            values: Some(Values::Bool(3)),
            null: false,
        };
        if row.owner != state.owner()
            || fact.owner != state.owner()
            || row.after != before
            || scalar.control
            || scalar.domain != full
        {
            return Err(invalid(span));
        }
        let Some(version) = scalar.version else {
            return Ok(before);
        };
        if !flow.spend(state.cells.depth() + 4) {
            return Err(budget(span));
        }
        let cell = state.get(version.local).ok_or_else(|| invalid(span))?;
        let Data::Initialized {
            version: Some(current),
            domain: Some(domain),
            origin,
        } = &cell.data
        else {
            return Err(invalid(span));
        };
        if version.owner != state.owner()
            || version != *current
            || *domain != full
            || *origin != scalar.origin
            || cell.control
        {
            return Err(invalid(span));
        }
        let cell = Cell {
            data: Data::Initialized {
                version: Some(version),
                domain: Some(Domain::boolean(value)),
                origin: *origin,
            },
            control: false,
        };
        let mut after = state.snapshot(flow, span, self.room())?;
        after.put(version.local, &cell, flow, span, self.room())?;
        self.keep(after, span)
    }
}

#[cfg(test)]
mod tests;
