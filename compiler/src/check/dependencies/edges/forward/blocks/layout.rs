use super::*;
use crate::check::dependencies::bodies::{
    Completion, Layout, completion::Shape, layout::MAX_SLOTS,
};

impl Checker {
    pub(super) fn validate_result_layout(
        &mut self,
        id: crate::hir::BlockId,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof result-layout budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof result-layout identity mismatch", span);
        if !self
            .flow
            .spend(self.bodies.len().checked_ilog2().unwrap_or(0) as usize + 3)
        {
            return Err(budget());
        }
        let body = self.bodies.get(&id).ok_or_else(invalid)?;
        if !body.completion.valid() {
            return Err(invalid());
        }
        let shape = body.completion.result;
        let Layout::Slots(slots) = &body.layout else {
            return if matches!(
                (&body.layout, shape),
                (Layout::Stopped, Shape::Never)
                    | (Layout::Unknown, Shape::Other | Shape::Union { .. })
            ) {
                Ok(())
            } else {
                Err(invalid())
            };
        };
        if slots.len() > MAX_SLOTS || !self.flow.spend(slots.len() + 1) {
            return Err(budget());
        }
        let (count, names) = body.layout.counts().ok_or_else(invalid)?;
        if !self
            .flow
            .spend((count + names) * (count.checked_ilog2().unwrap_or(0) as usize + 3))
        {
            return Err(budget());
        }
        let expected = match shape {
            Shape::Record { fields } => fields + 1,
            Shape::Scalar(_) | Shape::List { .. } | Shape::Reference(_) => 1,
            _ => return Err(invalid()),
        };
        if count != expected || slots[0].field.is_some() || slots[0].mutable {
            return Err(invalid());
        }
        if !matches!(shape, Shape::Record { .. }) && slots[0].shape != shape {
            return Err(invalid());
        }
        let mut fields = BTreeSet::new();
        for (index, slot) in slots.iter().enumerate() {
            if !(Completion {
                normal: slot.shape != Shape::Never,
                result: slot.shape,
            })
            .valid()
                || (index != 0
                    && !slot
                        .field
                        .as_ref()
                        .is_some_and(|field| !field.is_empty() && fields.insert(field.as_str())))
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
