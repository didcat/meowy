use super::*;
use crate::check::dependencies::bodies::{Layout, completion::Shape};

impl Checker {
    pub(in super::super) fn scalar_scope_type(
        &mut self,
        block: hir::BlockId,
        span: Span,
    ) -> Result<Option<Scalar>> {
        let body = self.bodies.get(&block).ok_or_else(|| invalid(span))?;
        if let Some(scalar) = body.scalar {
            return Ok(Some(scalar));
        }
        if !matches!(body.completion.result, Shape::Record { .. })
            || self.proofs.dispatches.contains(&block)
        {
            return Ok(None);
        }
        self.validate_result_layout(block, span)?;
        let Layout::Slots(slots) = &self.bodies[&block].layout else {
            return Err(invalid(span));
        };
        if !self.flow.spend(slots.len() + 1) {
            return Err(budget(span));
        }
        for slot in slots {
            let Shape::Scalar(kind) = slot.shape else {
                return Ok(None);
            };
            if slot.mutable
                || (Scalar {
                    kind,
                    nullable: false,
                })
                .shape()
                .is_none()
            {
                return Ok(None);
            }
        }
        let Shape::Scalar(kind) = slots[0].shape else {
            return Err(invalid(span));
        };
        Ok(Some(Scalar {
            kind,
            nullable: false,
        }))
    }
}

#[cfg(test)]
mod tests;
