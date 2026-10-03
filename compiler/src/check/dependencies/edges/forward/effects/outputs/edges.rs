use super::*;

impl Checker {
    pub(in super::super::super) fn validate_output_edges(
        &mut self,
        id: PointId,
        owner: usize,
        span: Span,
    ) -> Result<()> {
        let invalid = || Diagnostic::unsupported("proof output-effect identity mismatch", span);
        let budget = || Diagnostic::unsupported("proof output-effect budget exhausted", span);
        if !self
            .flow
            .spend(self.outputs.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let op = self.outputs.get(&id).ok_or_else(invalid)?;
        let limit = crate::check::dependencies::sequences::MAX_ITEMS;
        if op.parts.len() > limit
            || op.edges.len() > limit * 3 + 2
            || !self.flow.spend(op.parts.len() * 8 + op.edges.len() + 10)
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if op.owner != owner
            || point.owner != owner
            || point.kind != PointKind::Expr
            || !point.complete
            || point.span != op.span
            || op
                .stopped
                .is_some_and(|part| op.parts.get(part).is_none_or(Option::is_none))
        {
            return Err(invalid());
        }
        for input in op.parts.iter().flatten() {
            if input.point == id
                || !self.points.get(input.point).is_some_and(|child| {
                    child.complete
                        && child.owner == owner
                        && child.parent == Some(id)
                        && child.block == point.block
                        && matches!(child.kind, PointKind::Expr | PointKind::And | PointKind::Or)
                })
            {
                return Err(invalid());
            }
        }
        let mut edges = op.edges.iter();
        let mut take = |from, to, route| edges.next() == Some(&Edge::new(from, to, route));
        let (mut from, mut route) = (Port::Entry(id), Route::Next);
        if op.panic {
            if !take(from, Port::Prefix(id), route) {
                return Err(invalid());
            }
            (from, route) = (Port::Prefix(id), Route::Returned);
        }
        for (part, input) in op.parts.iter().enumerate() {
            if let Some(input) = input {
                if !take(from, Port::Entry(input.point), route) {
                    return Err(invalid());
                }
                (from, route) = (Port::Normal(input.point), Route::Next);
                if input.primary {
                    let projection = Port::Projection {
                        point: id,
                        step: part,
                    };
                    if !take(from, projection, route) {
                        return Err(invalid());
                    }
                    from = projection;
                }
            }
            if op.stopped == Some(part) {
                break;
            }
            let output = Port::Output { point: id, part };
            if !take(from, output, route) {
                return Err(invalid());
            }
            (from, route) = (output, Route::Returned);
        }
        if op.stopped.is_none()
            && (!take(from, Port::Operation(id), route)
                || (!op.panic && !take(Port::Operation(id), Port::Normal(id), Route::Returned)))
        {
            return Err(invalid());
        }
        if edges.next().is_some() {
            return Err(invalid());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
