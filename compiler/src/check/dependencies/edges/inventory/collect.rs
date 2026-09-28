use super::*;

impl Checker {
    pub(crate) fn edge_inventory(&mut self, span: Span) -> Result<Vec<(Family, Edge)>> {
        let budget = || Diagnostic::unsupported("proof edge-inventory budget exhausted", span);
        let counts = self.edge_counts();
        let total = counts
            .iter()
            .try_fold(0usize, |total, (_, count)| total.checked_add(*count))
            .filter(|total| *total <= MAX_EDGES)
            .ok_or_else(budget)?;
        if !self.flow.spend(FAMILIES * 2) {
            return Err(budget());
        }
        let mut result = Vec::with_capacity(total);
        let mut actual = [0usize; FAMILIES];
        let mut append = |family: Family, edges: &[Edge]| -> Result<()> {
            let size = result
                .len()
                .checked_add(edges.len())
                .filter(|size| *size <= MAX_EDGES)
                .ok_or_else(budget)?;
            if !self.flow.spend(edges.len() + 1) {
                return Err(budget());
            }
            let index = family as usize;
            actual[index] = actual[index].checked_add(edges.len()).ok_or_else(budget)?;
            if actual[index] > counts[index].1 {
                return Err(Diagnostic::unsupported(
                    format!("proof edge-inventory count mismatch for {family:?}"),
                    span,
                ));
            }
            result.extend(edges.iter().map(|edge| (family, *edge)));
            debug_assert_eq!(result.len(), size);
            Ok(())
        };
        for value in self.branch_edges.values() {
            append(Family::Branch, value)?;
        }
        for value in self.region_edges.values() {
            append(Family::Region, value)?;
        }
        for value in self.sequences.values() {
            append(Family::Sequence, &value.edges)?;
        }
        for value in self.scope_exits.values() {
            append(Family::ScopeExit, std::slice::from_ref(&value.edge))?;
        }
        for value in self.endpoints.values() {
            append(Family::Endpoint, value)?;
        }
        for value in self.restart_edges.values() {
            append(Family::Restart, std::slice::from_ref(value))?;
        }
        for value in self.operations.values() {
            append(Family::Storage, &value.edges)?;
        }
        for value in self.local_reads.values() {
            append(Family::LocalRead, &value.edges)?;
        }
        for value in self.scalar_leaves.values() {
            append(Family::ScalarLeaf, &value.edges)?;
        }
        for value in self.heap_leaves.values() {
            append(Family::HeapLeaf, &value.edges)?;
        }
        for value in self.paths.values() {
            append(Family::Path, &value.edges)?;
        }
        for value in self.stores.values() {
            append(Family::Store, &value.edges)?;
        }
        for value in self.invocations.values() {
            append(Family::Invocation, &value.edges)?;
        }
        for value in self.indices.values() {
            append(Family::Index, &value.edges)?;
        }
        for value in self.methods.values() {
            append(Family::Method, &value.edges)?;
        }
        for value in self.elements.values() {
            append(Family::Element, &value.edges)?;
        }
        for value in self.exclusives.values() {
            append(Family::Exclusive, &value.edges)?;
        }
        for value in self.outputs.values() {
            append(Family::Output, &value.edges)?;
        }
        for value in self.unaries.values() {
            append(Family::Unary, &value.edges)?;
        }
        for value in self.derefs.values() {
            append(Family::Deref, &value.edges)?;
        }
        for value in self.fields.values() {
            append(Family::Field, &value.edges)?;
        }
        for value in self.typed_ops.values() {
            append(Family::Typed, &value.edges)?;
        }
        for value in self.coercions.values() {
            append(Family::Coercion, &value.edges)?;
        }
        for value in self.narrowings.values() {
            append(Family::Narrowing, &value.edges)?;
        }
        for value in self.binaries.values() {
            append(Family::Binary, &value.edges)?;
        }
        for value in self.dispatch_ops.values() {
            append(Family::Dispatch, &value.edges)?;
        }
        for value in self.reborrow_ops.values() {
            append(Family::Reborrow, &value.edges)?;
        }
        for value in self.projections.values() {
            append(Family::Projection, &value.edges)?;
        }
        for value in self.place_borrows.values() {
            append(Family::PlaceBorrow, &value.edges)?;
        }
        for value in self.temporary_borrows.values() {
            append(Family::TemporaryBorrow, &value.edges)?;
        }
        for value in self.emissions.values() {
            append(Family::Emission, &value.edges)?;
        }
        for (family, count) in counts {
            if actual[family as usize] != count {
                return Err(Diagnostic::unsupported(
                    format!("proof edge-inventory count mismatch for {family:?}"),
                    span,
                ));
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
