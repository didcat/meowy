use super::PointKind;
use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir,
};

pub(crate) const MAX_ITEMS: usize = 262_144;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Materialize {
        local: hir::LocalId,
        statement: hir::StatementId,
    },
    Field(usize),
    Load(hir::ReferenceMode),
    Address(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Projection {
    pub(crate) owner: usize,
    pub(crate) parent: hir::PointId,
    pub(crate) steps: Vec<Step>,
    pub(crate) site: Option<hir::ReborrowId>,
    pub(crate) mode: Option<hir::ReferenceMode>,
    pub(crate) control: bool,
    pub(crate) span: Span,
}

impl Checker {
    pub(crate) fn projection_step(&mut self, plan: &mut Projection, step: Step) -> Result<()> {
        if plan.steps.len() >= crate::list::MAX_WRITE_PATH || !self.flow.spend(1) {
            return Err(Diagnostic::unsupported(
                "proof borrow-projection budget exhausted",
                plan.span,
            ));
        }
        plan.steps.push(step);
        Ok(())
    }

    pub(crate) fn capture_projection(&mut self, id: hir::PointId, plan: Projection) -> Result<()> {
        let budget =
            || Diagnostic::unsupported("proof borrow-projection budget exhausted", plan.span);
        let invalid =
            || Diagnostic::unsupported("proof borrow-projection identity mismatch", plan.span);
        if plan.steps.len() > crate::list::MAX_WRITE_PATH
            || !self.flow.spend(
                plan.steps.len()
                    + self.projections.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + self.proofs.temporaries.len().checked_ilog2().unwrap_or(0) as usize
                    + 4,
            )
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if point.kind != PointKind::Expr
            || point.owner != self.owner
            || plan.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || !self.points.get(plan.parent).is_some_and(|parent| {
                parent.parent == Some(id)
                    && parent.owner == self.owner
                    && parent.block == point.block
                    && parent.complete
                    && matches!(
                        parent.kind,
                        PointKind::Expr | PointKind::And | PointKind::Or
                    )
            })
            || plan.site.is_some() != plan.mode.is_some()
            || plan.site.is_some_and(|site| site >= self.reborrows)
            || (plan.site.is_none() && !plan.steps.is_empty())
        {
            return Err(invalid());
        }
        let mut address = false;
        for (index, step) in plan.steps.iter().enumerate() {
            match step {
                Step::Materialize { local, statement } => {
                    if index != 0
                        || self.locals.get(*local).is_none()
                        || self.proofs.temporaries.get(local) != Some(statement)
                    {
                        return Err(invalid());
                    }
                }
                Step::Field(field) => {
                    if address || *field >= crate::borrow_value::MAX_PARTS {
                        return Err(invalid());
                    }
                }
                Step::Load(_) => {
                    if address {
                        return Err(invalid());
                    }
                }
                Step::Address(field) => {
                    if *field >= crate::borrow_value::MAX_PARTS {
                        return Err(invalid());
                    }
                    address = true;
                }
            }
        }
        if let Some(prior) = self.projections.get(&id) {
            return if *prior == plan {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        let count = self
            .projection_items
            .checked_add(plan.steps.len() + 1)
            .filter(|count| *count <= MAX_ITEMS)
            .ok_or_else(budget)?;
        self.projections.insert(id, plan);
        self.projection_items = count;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
