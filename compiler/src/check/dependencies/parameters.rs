use super::{
    bodies::completion::{Completion, Shape},
    edges::MAX_EDGES,
    sequences::MAX_ITEMS,
};
use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Parameter {
    pub(crate) function: hir::FunctionId,
    pub(crate) position: usize,
    pub(crate) shape: Shape,
}

impl Checker {
    pub(crate) fn capture_parameters(
        &mut self,
        function: hir::FunctionId,
        ids: &[hir::LocalId],
        span: Span,
    ) -> Result<()> {
        self.capture_parameters_limited(function, ids, span, MAX_EDGES)
    }

    pub(super) fn capture_parameters_limited(
        &mut self,
        function: hir::FunctionId,
        ids: &[hir::LocalId],
        span: Span,
        limit: usize,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof parameter-shape budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof parameter-shape identity mismatch", span);
        if ids.len() > MAX_ITEMS {
            return Err(budget());
        }
        let size = self
            .parameters
            .len()
            .checked_add(ids.len())
            .filter(|size| *size <= limit.min(MAX_EDGES))
            .ok_or_else(budget)?;
        if !self.flow.spend(
            ids.len()
                * (size.checked_ilog2().unwrap_or(0) as usize * 4
                    + self.proofs.bindings.len().checked_ilog2().unwrap_or(0) as usize
                    + 8)
                + 1,
        ) {
            return Err(budget());
        }
        if function.checked_add(1) != Some(self.owner) || self.functions.get(function).is_none() {
            return Err(invalid());
        }
        let mut values = BTreeMap::new();
        for (position, &local) in ids.iter().enumerate() {
            let ty = self.locals.get(local).ok_or_else(invalid)?;
            let value = Parameter {
                function,
                position,
                shape: Completion::of(ty).result,
            };
            if !self.proofs.bindings.contains_key(&local)
                || self.parameters.contains_key(&local)
                || values.insert(local, value).is_some()
            {
                return Err(invalid());
            }
        }
        self.parameters.extend(values);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
