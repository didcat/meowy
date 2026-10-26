use super::*;

impl Snapshots {
    pub(super) fn record_read_expression(
        &mut self,
        target: (PointId, hir::BlockId),
        source: &(usize, Effect),
        observations: &observations::Observations<'_, '_>,
        flow: &mut Flow,
        span: Span,
    ) -> Result<()> {
        let (point, block) = target;
        let value = self.record_read_value(point, block, source, observations, flow, span)?;
        self.record_primary(
            point,
            block,
            Fact {
                owner: source.0,
                value,
            },
            flow,
            span,
        )
    }

    pub(self) fn record_read_value(
        &self,
        point: PointId,
        block: hir::BlockId,
        source: &(usize, Effect),
        observations: &observations::Observations<'_, '_>,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Option<Scalar>> {
        if !flow.spend(9) {
            return Err(budget(span));
        }
        let &(
            owner,
            Effect::Read {
                local,
                storage,
                normal,
                control,
            },
        ) = source
        else {
            return Err(invalid(span));
        };
        let port = Port::Normal(point);
        let row = self.rows.get(&port).ok_or_else(|| invalid(span))?;
        let op = self.rows.get(&Port::Operation(point));
        let location = observations
            .definitions
            .declarations
            .get(&local)
            .ok_or_else(|| invalid(span))?;
        let versions::Definition::Bind(bind) = location.version.definition else {
            return Err(invalid(span));
        };
        let record = observations
            .definitions
            .records
            .get(&bind)
            .ok_or_else(|| invalid(span))?;
        if !normal
            || local != storage
            || row.owner != owner
            || row.before != row.after
            || location.version.owner != owner
            || location.version.local != local
            || record.block != block
            || record.storage.version != location.version
            || observations.forwarded.contains_key(&port)
            || observations.logical.contains_key(&port)
            || op.is_some_and(|op| {
                op.owner != owner
                    || op.before != op.after
                    || op.after != row.before
                    || op.control != row.control
            })
        {
            return Err(invalid(span));
        }
        if op.is_none() {
            return Ok(None);
        }
        let Value::Known(id) = row.after else {
            return Ok(None);
        };
        let state = self.states.get(id).ok_or_else(|| invalid(span))?;
        if state.owner() != owner {
            return Err(invalid(span));
        }
        if state.reach == FALSE {
            return Ok(Some(Scalar {
                domain: Domain::BOTTOM,
                local: None,
                version: None,
                origin: None,
                control: row.control.unwrap_or(true) || control,
            }));
        }
        let Some(history) = row.control else {
            return Ok(None);
        };
        if !flow.spend(state.cells.depth() + 1) {
            return Err(budget(span));
        }
        let Some(cell) = state.get(local) else {
            return Ok(None);
        };
        let Data::Initialized {
            version: Some(version),
            ..
        } = cell.data
        else {
            return Ok(None);
        };
        if version != location.version {
            return Err(invalid(span));
        }
        self.read(state, local, control || history, observations, flow, span)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
