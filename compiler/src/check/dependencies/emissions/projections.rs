use super::Projection;
use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir,
};

impl Checker {
    pub(crate) fn emission_projection(
        &self,
        from: Option<hir::LocalId>,
        value: &hir::Expr,
        field: Option<&str>,
        span: Span,
    ) -> Result<Projection> {
        let Some(from) = from else {
            return Ok(Projection::Value);
        };
        let invalid =
            || Diagnostic::unsupported("proof emission projection identity mismatch", span);
        let Some(hir::Type::Record { primary, fields }) = self.locals.get(from) else {
            return Err(invalid());
        };
        let (base, projection, ty) = match &value.kind {
            hir::ExprKind::Primary(base) if field.is_none() => {
                (base, Projection::Primary, primary.as_ref())
            }
            hir::ExprKind::Field { value: base, index } => {
                let source = fields.get(*index).ok_or_else(invalid)?;
                if field != Some(source.name.as_str()) {
                    return Err(invalid());
                }
                (base, Projection::Field(*index), &source.ty)
            }
            _ => return Err(invalid()),
        };
        if !matches!(base.kind, hir::ExprKind::Local(id) if id == from)
            || base.ty != self.locals[from]
            || &value.ty != ty
        {
            return Err(invalid());
        }
        Ok(projection)
    }
}
