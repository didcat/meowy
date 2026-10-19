use super::*;

impl Snapshots {
    pub(super) fn incoming(
        &mut self,
        graph: &graph::Graph<'_>,
        port: Port,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Value> {
        if !flow.spend(3) {
            return Err(budget(span));
        }
        let node = graph.nodes.get(&port).ok_or_else(|| invalid(span))?;
        if node.incoming.len() > MAX_EDGES {
            return Err(budget(span));
        }
        let seed = self.seeds.get(&port).copied();
        let mut first = seed;
        let mut same = true;
        let mut missing = false;
        let mut prior = None;
        for input in &node.incoming {
            if !flow.spend(6) {
                return Err(budget(span));
            }
            let edge = graph.edges.get(input.edge).ok_or_else(|| invalid(span))?.1;
            let key = (
                input.source.span.start,
                input.source.span.end,
                edge.from,
                input.edge,
            );
            let source = graph.nodes.get(&edge.from);
            let site = source
                .map(|node| &node.site)
                .or_else(|| graph.external.get(&edge.from));
            if prior.is_some_and(|prior| prior >= key)
                || edge.to != port
                || input.reached != source.is_some()
                || site != Some(&input.source)
                || input.source.owner != node.site.owner
            {
                return Err(invalid(span));
            }
            prior = Some(key);
            if !input.reached || edge.route == Route::Backedge {
                missing = true;
                continue;
            }
            let row = self.rows.get(&edge.from).ok_or_else(|| invalid(span))?;
            if row.owner != node.site.owner {
                return Err(invalid(span));
            }
            match row.after {
                Value::Missing => missing = true,
                Value::Known(id) => {
                    if self
                        .states
                        .get(id)
                        .is_none_or(|state| state.owner() != node.site.owner)
                    {
                        return Err(invalid(span));
                    }
                    same &= first.is_none_or(|first| first == id);
                    first.get_or_insert(id);
                }
            }
        }
        if missing {
            return Ok(Value::Missing);
        }
        if same {
            return Ok(first.map_or(Value::Missing, Value::Known));
        }
        let mut joined = seed;
        for input in &node.incoming {
            if !flow.spend(3) {
                return Err(budget(span));
            }
            let edge = graph.edges.get(input.edge).ok_or_else(|| invalid(span))?.1;
            let Value::Known(next) = self
                .rows
                .get(&edge.from)
                .ok_or_else(|| invalid(span))?
                .after
            else {
                return Err(invalid(span));
            };
            let Some(current) = joined else {
                joined = Some(next);
                continue;
            };
            if current == next {
                continue;
            }
            let state = self.states[current].join(&self.states[next], flow, span, self.room())?;
            let Value::Known(id) = self.keep(state, span)? else {
                return Err(invalid(span));
            };
            joined = Some(id);
        }
        Ok(joined.map_or(Value::Missing, Value::Known))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
