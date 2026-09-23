mod record_chains;
mod record_results;
mod views;

use super::{Checker, Diagnostic, Expr, Result};
use crate::check::dependencies::records::{MAX_DEPTH, MAX_FIELDS};
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
        self.call_result_cells(&expr.ty, expr, args, depth)
    }

    pub(crate) fn call_result_cells(
        &mut self,
        result: &Type,
        expr: &Expr,
        args: &[Expr],
        depth: usize,
    ) -> Result<Cells> {
        if matches!(result, Type::Reference(target) if Self::origin_record(target).is_some() && target.has_reference())
        {
            return self.call_record_locations(expr, args, depth, result);
        }
        let Some(result_depth) = self.shared_cell_depth(result, expr)? else {
            return Ok(Cells::default());
        };
        if result_depth < 2 || !self.origin_carrier(result, expr.span)? {
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
        let mut pending = args
            .iter()
            .map(|arg| (arg, &arg.ty, Vec::new()))
            .collect::<Vec<_>>();
        let mut visits = 0;
        while let Some((arg, ty, path)) = pending.pop() {
            visits += 1;
            if visits > MAX_FIELDS || path.len() > MAX_DEPTH || !self.flow.spend(path.len() + 1) {
                return Err(Diagnostic::unsupported(
                    "proof returned cell input budget exhausted",
                    expr.span,
                ));
            }
            if !ty.has_borrowed() {
                continue;
            }
            if let Some(Type::Record { primary, fields }) = Self::origin_record(ty) {
                if primary.has_borrowed() {
                    return Ok(Cells::default());
                }
                if pending.len() + fields.len() > MAX_FIELDS {
                    return Err(Diagnostic::unsupported(
                        "proof returned cell input capacity exhausted",
                        expr.span,
                    ));
                }
                for (index, field) in fields.iter().enumerate() {
                    let mut path = path.clone();
                    path.push(index);
                    pending.push((arg, &field.ty, path));
                }
                continue;
            }
            let record = self
                .call_shared_view(ty, expr)?
                .filter(|(view, _)| view.pointee().is_some_and(Type::has_borrowed));
            let source = if let Some((view, layers)) = record {
                let locations = if path.is_empty() {
                    self.reference_cell_at(arg, depth + 1)?
                } else {
                    self.record_source_cells_at(arg, &path, depth + 1)?
                };
                let (mut matched, locations) =
                    self.record_chain_cells(locations, ty, layers, expr, result)?;
                let Some(source) =
                    self.returned_record_cells(locations, view, expr, Some(result_depth), result)?
                else {
                    return Ok(Cells::default());
                };
                self.merge_returned_cells(&mut matched, source, expr)?;
                matched
            } else {
                let Some(arg_depth) = self.shared_cell_depth(ty, expr)? else {
                    return Ok(Cells::default());
                };
                if arg_depth < result_depth {
                    continue;
                }
                let layers = arg_depth - result_depth;
                let mut ty = ty;
                for _ in 0..layers {
                    ty = ty.pointee().unwrap();
                }
                if !crate::borrow_contract::returns::candidate(result, ty) {
                    continue;
                }
                let mut source = if path.is_empty() {
                    self.reference_cell_at(arg, depth + 1)?
                } else {
                    self.record_source_cells_at(arg, &path, depth + 1)?
                };
                for _ in 0..layers {
                    source = self.expand_reference_cells(source, expr)?;
                }
                source
            };
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
            if !target.has_borrowed()
                || (Self::origin_record(target).is_some() && target.has_reference())
            {
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

#[cfg(test)]
mod records;
