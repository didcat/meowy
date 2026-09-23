use super::{Checker, Diagnostic, Expr, Origins, Result, Type};

impl Checker {
    pub(super) fn record_call_field_origins(
        &mut self,
        value: &Expr,
        args: &[Expr],
        path: &[usize],
        depth: usize,
    ) -> Result<Origins> {
        if path.len() > super::super::MAX_DEPTH || !self.flow.spend(path.len() + 1) {
            return Err(Diagnostic::unsupported(
                "proof record call field budget exhausted",
                value.span,
            ));
        }
        if path.is_empty() {
            return Ok(Origins::default());
        }
        let mut ty = &value.ty;
        for index in path {
            let Some(Type::Record { fields, .. }) = Self::origin_record(ty) else {
                return Ok(Origins::default());
            };
            let Some(field) = fields.get(*index) else {
                return Ok(Origins::default());
            };
            ty = &field.ty;
        }
        if !matches!(ty, Type::Reference(target) if !target.has_borrowed()) {
            return Ok(Origins::default());
        }
        self.call_result_origins(ty, value.span, args, depth)
    }
}

#[cfg(test)]
mod tests;
