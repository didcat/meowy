use super::{Checker, Expr, Origins, Result, Type};

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
        let (value, path) = self.record_source_path(value, path)?;
        if path.is_empty() {
            return Ok(None);
        }
        let key = crate::check::dependencies::records::shapes::ShapeKey {
            fields: path,
            variants: Vec::new(),
        };
        self.shape_field_type(&value.ty, &key, value.span)
    }
}

#[cfg(test)]
mod tests;
