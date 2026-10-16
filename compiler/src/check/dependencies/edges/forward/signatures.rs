use super::{
    entries::{MAX_ENTRIES, Reports},
    *,
};
use crate::{
    check::dependencies::{
        Invocation,
        bodies::{Completion, completion::Shape},
        sequences::MAX_ITEMS,
    },
    hir,
};

mod parameters;
mod validation;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Index {
    pub(crate) functions: BTreeMap<hir::FunctionId, Signature>,
    pub(crate) params: BTreeMap<hir::LocalId, (hir::FunctionId, usize)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Signature {
    pub(crate) body: hir::BlockId,
    pub(crate) params: Vec<hir::LocalId>,
    pub(crate) shape: Shape,
}

impl Checker {
    pub(super) fn function_signatures(
        &mut self,
        program: &hir::Program,
        reports: &Reports,
        span: Span,
    ) -> Result<Index> {
        self.function_signatures_limited(program, reports, span, MAX_ENTRIES - 1, MAX_EDGES)
    }

    pub(self) fn function_signatures_limited(
        &mut self,
        program: &hir::Program,
        reports: &Reports,
        span: Span,
        limit: usize,
        parts: usize,
    ) -> Result<Index> {
        let budget = || Diagnostic::unsupported("proof function-signature budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof function-signature identity mismatch", span);
        if program.functions.len() > limit.min(MAX_ENTRIES - 1)
            || !self.flow.spend(program.functions.len() + 1)
        {
            return Err(budget());
        }
        let mut count = 0usize;
        for function in &program.functions {
            if function.params.len() > MAX_ITEMS || !self.flow.spend(1) {
                return Err(budget());
            }
            count = count
                .checked_add(function.params.len())
                .filter(|count| *count <= parts.min(MAX_EDGES) / 2)
                .ok_or_else(budget)?;
        }
        let root = self.signature_root(reports, program.functions.len(), span)?;
        if program.locals.len() != reports.locals || program.body.id != root {
            return Err(invalid());
        }
        let mut index = Index::default();
        for function in &program.functions {
            if !self.flow.spend(
                index.functions.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + reports.entries.len().checked_ilog2().unwrap_or(0) as usize
                    + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                    + function.params.len()
                    + 8,
            ) {
                return Err(budget());
            }
            let header = Signature {
                body: function.body.id,
                params: function.params.clone(),
                shape: Invocation::result_shape(&function.result),
            };
            let owner = function.id.checked_add(1).ok_or_else(invalid)?;
            if !(Completion {
                normal: header.shape != Shape::Never,
                result: header.shape,
            })
            .valid()
                || reports.entries.get(&owner).map(|(body, _)| *body) != Some(header.body)
                || !self
                    .bodies
                    .get(&header.body)
                    .is_some_and(|body| body.owner == owner && body.parent.is_none())
            {
                return Err(invalid());
            }
            for (position, &local) in header.params.iter().enumerate() {
                if !self.flow.spend(
                    index.params.len().checked_ilog2().unwrap_or(0) as usize * 2
                        + self.proofs.bindings.len().checked_ilog2().unwrap_or(0) as usize
                        + 4,
                ) {
                    return Err(budget());
                }
                if local >= reports.locals
                    || !self.proofs.bindings.contains_key(&local)
                    || index
                        .params
                        .insert(local, (function.id, position))
                        .is_some()
                {
                    return Err(invalid());
                }
            }
            if index.functions.insert(function.id, header).is_some() {
                return Err(invalid());
            }
        }
        Ok(index)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;

#[cfg(test)]
mod limits;

#[cfg(test)]
mod construction;
