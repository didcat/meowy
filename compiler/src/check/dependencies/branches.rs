use super::{PointKind, edges::MAX_EDGES};
use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir::PointId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Branch {
    pub(crate) owner: usize,
    pub(crate) input: PointId,
    pub(crate) normal: bool,
    pub(crate) control: bool,
}

impl Checker {
    pub(crate) fn branch_input(
        &mut self,
        id: PointId,
        input: PointId,
        normal: bool,
        control: bool,
        span: Span,
    ) -> Result<()> {
        if self.required {
            return Ok(());
        }
        let budget = || Diagnostic::unsupported("proof branch-input budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof branch-input identity mismatch", span);
        if !self.flow.spend(7) {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        let source = self.points.get(input).ok_or_else(invalid)?;
        let condition = source.parent.ok_or_else(invalid)?;
        if !matches!(
            point.kind,
            PointKind::Match | PointKind::And | PointKind::Or
        ) || point.owner != self.owner
            || !point.complete && self.point != Some(id)
            || !self.points.get(condition).is_some_and(|test| {
                test.kind == PointKind::Condition
                    && test.parent == Some(id)
                    && test.owner == point.owner
                    && test.block == point.block
                    && test.complete
            })
            || !matches!(
                source.kind,
                PointKind::Expr | PointKind::And | PointKind::Or
            )
            || source.owner != point.owner
            || source.block != point.block
            || !source.complete
        {
            return Err(invalid());
        }
        let branch = Branch {
            owner: point.owner,
            input,
            normal,
            control,
        };
        if let Some(prior) = self.branch_inputs.get(&id) {
            return if *prior == branch {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if self.branch_inputs.len() >= MAX_EDGES {
            return Err(budget());
        }
        self.branch_inputs.insert(id, branch);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
