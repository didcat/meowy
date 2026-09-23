use super::{Checker, Diagnostic, Expr, Result};
use crate::check::dependencies::{Cells, references::MAX_ROOTS};
use crate::hir::Type;

impl Checker {
    pub(crate) fn call_reference_cells(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        depth: usize,
    ) -> Result<Cells> {
        let Type::Reference(inner) = &expr.ty else {
            return Ok(Cells::default());
        };
        if !matches!(inner.as_ref(), Type::Reference(leaf) if !leaf.has_borrowed())
            || !Self::origin_reference(inner)
        {
            return Ok(Cells::default());
        }
        if !self.flow.spend(args.len() + 1) {
            return Err(Diagnostic::unsupported(
                "proof reference cell call budget exhausted",
                expr.span,
            ));
        }
        let mut cells = Cells::default();
        let mut found = false;
        for arg in args {
            let supported = !arg.ty.has_borrowed()
                || matches!(&arg.ty, Type::Reference(target) if !target.has_borrowed()
                    || matches!(target.as_ref(), Type::Reference(leaf) if !leaf.has_borrowed()));
            if !supported {
                return Ok(Cells::default());
            }
            if !crate::borrow_contract::returns::candidate(&expr.ty, &arg.ty) {
                continue;
            }
            let source = self.reference_cell_at(arg, depth + 1)?;
            let work = source
                .places
                .iter()
                .map(|(_, path)| path.len() + 1)
                .sum::<usize>()
                + 1;
            if !self.flow.spend(work) {
                return Err(Diagnostic::unsupported(
                    "proof reference cell call budget exhausted",
                    expr.span,
                ));
            }
            cells.complete = if found {
                cells.complete && source.complete
            } else {
                source.complete
            };
            found = true;
            cells.places.extend(source.places);
            if cells.places.len() > MAX_ROOTS {
                return Err(Diagnostic::unsupported(
                    "proof reference cell call capacity exhausted",
                    expr.span,
                ));
            }
        }
        Ok(cells)
    }
}

#[cfg(test)]
mod tests;
