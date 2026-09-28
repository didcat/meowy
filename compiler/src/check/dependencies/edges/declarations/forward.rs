use super::*;

impl Checker {
    pub(crate) fn forward_group_endpoint(
        &mut self,
        id: PointId,
        functions: &[crate::hir::FunctionId],
        span: Span,
    ) -> Result<()> {
        let invalid =
            || Diagnostic::unsupported("proof forward group endpoint identity mismatch", span);
        let budget =
            || Diagnostic::unsupported("proof forward group endpoint budget exhausted", span);
        if functions.len() > crate::flow::MAX_NODES
            || !self.flow.spend(
                functions
                    .len()
                    .saturating_mul(self.bodies.len().checked_ilog2().unwrap_or(0) as usize + 4)
                    + self.endpoints.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + 4,
            )
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if point.kind != PointKind::Stmt
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || functions.is_empty()
            || !functions.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err(invalid());
        }
        for &function in functions {
            let owner = function.checked_add(1).ok_or_else(invalid)?;
            let item = self
                .functions
                .get(function)
                .and_then(Option::as_ref)
                .ok_or_else(invalid)?;
            if item.id != function
                || owner == self.owner
                || !self
                    .bodies
                    .get(&item.body.id)
                    .is_some_and(|body| body.owner == owner && body.parent.is_none())
            {
                return Err(invalid());
            }
        }
        self.publish_declaration_endpoint(id, span, "forward group")
    }
}

#[cfg(test)]
mod tests;
