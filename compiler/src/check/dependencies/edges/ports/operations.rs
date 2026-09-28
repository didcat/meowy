use super::*;
use std::collections::BTreeMap;

impl Checker {
    pub(crate) fn operation_port_owners(&mut self, span: Span) -> Result<BTreeMap<PointId, usize>> {
        let budget = || Diagnostic::unsupported("proof operation-port budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof operation-port identity mismatch", span);
        if !self.flow.spend(self.endpoints.len()) {
            return Err(budget());
        }
        let mut result = BTreeMap::new();
        let mut record = |id: PointId, owner: usize, edges: &[Edge]| -> Result<()> {
            if edges.len() > MAX_EDGES || !self.flow.spend(edges.len() + 1) {
                return Err(budget());
            }
            let port = Port::Operation(id);
            if !edges
                .iter()
                .any(|edge| edge.from == port || edge.to == port)
            {
                return Ok(());
            }
            if !self
                .points
                .get(id)
                .is_some_and(|point| point.complete && point.owner == owner)
            {
                return Err(invalid());
            }
            if !self
                .flow
                .spend(result.len().checked_ilog2().unwrap_or(0) as usize * 2 + 3)
            {
                return Err(budget());
            }
            if let Some(prior) = result.get(&id) {
                return if *prior == owner {
                    Ok(())
                } else {
                    Err(invalid())
                };
            }
            if result.len() >= MAX_EDGES {
                return Err(budget());
            }
            result.insert(id, owner);
            Ok(())
        };
        for (id, op) in &self.operations {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.local_reads {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.scalar_leaves {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.heap_leaves {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.paths {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.stores {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.indices {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.methods {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.elements {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.exclusives {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.outputs {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.unaries {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.derefs {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.fields {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.typed_ops {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.coercions {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.narrowings {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.binaries {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.dispatch_ops {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.reborrow_ops {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.projections {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.place_borrows {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.temporary_borrows {
            record(*id, op.owner, &op.edges)?;
        }
        for (id, op) in &self.emissions {
            record(*id, op.owner, &op.edges)?;
        }
        for call in self.invocations.values() {
            record(call.point, call.owner, &call.edges)?;
        }
        for (source, edges) in &self.endpoints {
            if let crate::check::SequenceSource::Expr(id) = source {
                let owner = self.points.get(*id).ok_or_else(invalid)?.owner;
                record(*id, owner, edges)?;
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
