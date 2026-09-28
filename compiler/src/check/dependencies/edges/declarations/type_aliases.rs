use super::*;

impl Checker {
    pub(crate) fn type_alias_endpoint(&mut self, id: PointId, span: Span) -> Result<()> {
        let invalid =
            || Diagnostic::unsupported("proof type alias endpoint identity mismatch", span);
        if !self
            .flow
            .spend(self.endpoints.len().checked_ilog2().unwrap_or(0) as usize * 2 + 4)
        {
            return Err(Diagnostic::unsupported(
                "proof type alias endpoint budget exhausted",
                span,
            ));
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if point.kind != PointKind::Stmt
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
        {
            return Err(invalid());
        }
        self.publish_declaration_endpoint(id, span, "type alias")
    }
}

#[cfg(test)]
mod tests;
