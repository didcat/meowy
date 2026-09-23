use super::{Checker, Diagnostic, Expr, Result};
use crate::check::dependencies::{
    Cells,
    references::{MAX_CELL_DEPTH, MAX_ROOTS},
};
use crate::hir::Type;

impl Checker {
    pub(crate) fn call_reference_cells(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        depth: usize,
    ) -> Result<Cells> {
        let Some(result_depth) = self.shared_cell_depth(&expr.ty, expr)? else {
            return Ok(Cells::default());
        };
        if result_depth < 2 || !self.origin_carrier(&expr.ty, expr.span)? {
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
            if !arg.ty.has_borrowed() {
                continue;
            }
            let Some(arg_depth) = self.shared_cell_depth(&arg.ty, expr)? else {
                return Ok(Cells::default());
            };
            if arg_depth < result_depth {
                continue;
            }
            let layers = arg_depth - result_depth;
            let mut ty = &arg.ty;
            for _ in 0..layers {
                ty = ty.pointee().unwrap();
            }
            if !crate::borrow_contract::returns::candidate(&expr.ty, ty) {
                continue;
            }
            let mut source = self.reference_cell_at(arg, depth + 1)?;
            for _ in 0..layers {
                source = self.expand_reference_cells(source, expr)?;
            }
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

    pub(super) fn shared_cell_depth(
        &mut self,
        mut ty: &Type,
        expr: &Expr,
    ) -> Result<Option<usize>> {
        let mut depth = 0;
        loop {
            self.origin_visit(expr)?;
            let Type::Reference(target) = ty else {
                return Ok(None);
            };
            depth += 1;
            if depth > MAX_CELL_DEPTH + 1 {
                return Err(Diagnostic::unsupported(
                    "proof returned cell type depth exhausted",
                    expr.span,
                ));
            }
            if !target.has_borrowed() {
                return Ok(Some(depth));
            }
            ty = target;
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod chains;
