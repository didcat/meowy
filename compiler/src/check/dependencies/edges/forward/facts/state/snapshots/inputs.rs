use super::*;

impl Snapshots {
    #[cfg(test)]
    pub(super) fn incoming(
        &mut self,
        graph: &graph::Graph<'_>,
        port: Port,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Value> {
        self.incoming_with(graph, port, None, flow, span)
    }

    #[cfg(test)]
    pub(super) fn incoming_with(
        &mut self,
        graph: &graph::Graph<'_>,
        port: Port,
        absent: Option<&HashMap<Port, usize>>,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Value> {
        self.inputs(graph, port, absent, flow, span)
            .map(|(value, _)| value)
    }

    pub(super) fn inputs(
        &mut self,
        graph: &graph::Graph<'_>,
        port: Port,
        absent: Option<&HashMap<Port, usize>>,
        flow: &mut Flow,
        span: Span,
    ) -> Result<(Value, Option<bool>)> {
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
        let mut control = Some(false);
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
            if edge.route == Route::Backedge {
                missing = true;
                continue;
            }
            if !input.reached {
                if let Some(owner) = absent.and_then(|absent| absent.get(&edge.from)) {
                    if *owner != node.site.owner {
                        return Err(invalid(span));
                    }
                } else {
                    missing = true;
                }
                continue;
            }
            let row = self.rows.get(&edge.from).ok_or_else(|| invalid(span))?;
            if row.owner != node.site.owner {
                return Err(invalid(span));
            }
            match row.after {
                Value::Missing => {
                    missing = true;
                    control = history::combine(control, row.control);
                }
                Value::Known(id) => {
                    let state = self.states.get(id).ok_or_else(|| invalid(span))?;
                    if state.owner() != node.site.owner {
                        return Err(invalid(span));
                    }
                    if state.reach != FALSE {
                        control = history::combine(control, row.control);
                    }
                    if node.incoming.len() == 1
                        && seed.is_none()
                        && matches!(edge.route, Route::True | Route::False)
                    {
                        let value = self.branch(edge, row.after, flow, span)?;
                        return Ok((value, history::combine(control, None)));
                    }
                    same &= first.is_none_or(|first| first == id);
                    first.get_or_insert(id);
                }
            }
        }
        if missing {
            return Ok((Value::Missing, history::combine(control, None)));
        }
        if same {
            return Ok((first.map_or(Value::Missing, Value::Known), control));
        }
        let mut joined = seed;
        for input in &node.incoming {
            if !flow.spend(3) {
                return Err(budget(span));
            }
            if !input.reached {
                continue;
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
        Ok((joined.map_or(Value::Missing, Value::Known), control))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
