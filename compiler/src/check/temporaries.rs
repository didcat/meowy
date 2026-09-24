use super::{Checker, Result};
use crate::ast::Span;
use crate::diagnostic::Diagnostic;
use crate::hir::{Expr, ExprKind, Type};

impl Checker {
    pub(crate) fn temporary_borrow(&mut self, value: Expr, span: Span) -> Result<Expr> {
        if value.ty == Type::Never {
            return Ok(value);
        }
        crate::borrow_contract::type_weight(&value.ty, &mut self.flow, span)?;
        let Some((statement, used)) = self.statement.last_mut() else {
            return Err(Diagnostic::unsupported(
                "temporary outside a complete statement",
                span,
            ));
        };
        *used = true;
        let statement = *statement;
        if self.proofs.temporaries.len() >= 65_536 || !self.flow.spend(1) {
            return Err(Diagnostic::unsupported(
                "temporary lifetime budget exhausted",
                span,
            ));
        }
        let id = self.local(value.ty.clone());
        self.track_reference(id, &value, false)?;
        self.track_reference_cell(id, &value, false)?;
        self.track_record_references(id, &value, false)?;
        self.capture_record_shapes(id, &value)?;
        if self.control || self.derived_expr(&value) {
            self.mark_derived(id);
        }
        self.proofs.temporaries.insert(id, statement);
        Ok(Expr {
            ty: self.reference_type(value.ty.clone(), span)?,
            kind: ExprKind::TemporaryBorrow {
                id,
                statement,
                value: Box::new(value),
            },
            span,
        })
    }
}
