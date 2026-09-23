use super::{Checker, Diagnostic, Expr, Origins, Result, Type};

impl Checker {
    pub(super) fn record_call_field_origins(
        &mut self,
        value: &Expr,
        args: &[Expr],
        path: &[usize],
        depth: usize,
    ) -> Result<Origins> {
        let Some(ty) = self.record_call_field_type(value, path)? else {
            return Ok(Origins::default());
        };
        if !matches!(ty, Type::Reference(target) if !target.has_borrowed()) {
            return Ok(Origins::default());
        }
        self.call_result_origins(ty, value.span, args, depth)
    }

    pub(crate) fn record_call_field_type<'a>(
        &mut self,
        value: &'a Expr,
        path: &[usize],
    ) -> Result<Option<&'a Type>> {
        if path.len() > super::super::MAX_DEPTH || !self.flow.spend(path.len() + 1) {
            return Err(Diagnostic::unsupported(
                "proof record call field budget exhausted",
                value.span,
            ));
        }
        if path.is_empty() {
            return Ok(None);
        }
        let mut ty = &value.ty;
        for index in path {
            let Some(Type::Record { fields, .. }) = Self::origin_record(ty) else {
                return Ok(None);
            };
            let Some(field) = fields.get(*index) else {
                return Ok(None);
            };
            ty = &field.ty;
        }
        Ok(Some(ty))
    }
}

#[cfg(test)]
mod tests;
