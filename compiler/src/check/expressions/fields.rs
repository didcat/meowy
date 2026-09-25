use super::*;

impl Checker {
    pub(crate) fn field_point(
        &mut self,
        receiver: &ast::Expr,
        name: &str,
        span: Span,
    ) -> Result<(hir::PointId, bool, hir::Expr)> {
        let (input, mut value) = self.expr_point(receiver, None)?;
        let load = matches!(value.ty, Type::Reference(_));
        if let Type::Reference(ty) = &value.ty {
            value = hir::Expr {
                ty: *ty.clone(),
                span: value.span,
                kind: hir::ExprKind::Deref(Box::new(value)),
            };
        }
        let Type::Record { fields, .. } = &value.ty else {
            return Err(Self::error(
                "E201",
                format!("type {:?} has no field `{name}`", value.ty),
                span,
            ));
        };
        let (index, field) = fields
            .iter()
            .enumerate()
            .find(|(_, field)| field.name == name)
            .ok_or_else(|| Self::error("E201", format!("unknown record field `{name}`"), span))?;
        let ty = field.ty.clone();
        Ok((
            input,
            load,
            hir::Expr {
                kind: hir::ExprKind::Field {
                    value: Box::new(value),
                    index,
                },
                ty,
                span,
            },
        ))
    }
}

#[cfg(test)]
mod tests;
