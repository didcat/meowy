use super::{Cells, Checker, Diagnostic, Expr, Result, Type};

impl Checker {
    pub(super) fn call_record_locations(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        depth: usize,
    ) -> Result<Cells> {
        if !self.flow.spend(args.len() + 1) {
            return Err(Diagnostic::unsupported(
                "proof returned record location budget exhausted",
                expr.span,
            ));
        }
        let mut cells = Cells::default();
        let mut found = false;
        for arg in args {
            let supported = arg.ty == expr.ty
                || !arg.ty.has_borrowed()
                || matches!(&arg.ty, Type::Reference(target) if !target.has_borrowed());
            if !supported {
                return Ok(Cells::default());
            }
            if !crate::borrow_contract::returns::candidate(&expr.ty, &arg.ty) {
                continue;
            }
            let source = self.reference_cell_at(arg, depth + 1)?;
            if !found {
                cells.complete = true;
                found = true;
            }
            self.merge_returned_cells(&mut cells, source, expr)?;
        }
        Ok(cells)
    }
}

#[cfg(test)]
mod tests;
