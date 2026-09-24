use super::super::{Cells, Checker, Diagnostic, Expr, MAX_DEPTH, MAX_FIELDS, Result, Type};
use crate::check::dependencies::records::shapes::ShapeKey;

impl Checker {
    pub(in crate::check::dependencies::calls::cells) fn hidden_union_cells(
        &mut self,
        locations: &Cells,
        prefix: &[usize],
        ty: &Type,
        result: &Type,
        expr: &Expr,
    ) -> Result<Option<Cells>> {
        let Some(paths) = self.hidden_union_paths(ty, result, expr)? else {
            return Ok(None);
        };
        let mut cells = Cells {
            complete: true,
            ..Cells::default()
        };
        for (key, layers) in paths {
            let mut source = self
                .location_shape_source(locations, prefix, &key, expr.span)?
                .cells;
            for _ in 0..layers {
                source = self.expand_reference_cells(source, expr)?;
            }
            self.merge_returned_cells(&mut cells, source, expr)?;
        }
        Ok(Some(cells))
    }

    pub(super) fn hidden_union_paths(
        &mut self,
        ty: &Type,
        result: &Type,
        expr: &Expr,
    ) -> Result<Option<Vec<(ShapeKey, usize)>>> {
        let Some(result_depth) = self.shared_cell_depth(result, expr)? else {
            return Ok(None);
        };
        let mut target = result;
        for _ in 0..result_depth {
            target = target.pointee().unwrap();
        }
        let mut pending = vec![(ty, Vec::new(), Vec::new())];
        let mut paths = Vec::new();
        let mut visits = 0;
        while let Some((ty, fields, variants)) = pending.pop() {
            visits += 1;
            if visits > MAX_FIELDS
                || fields.len() + variants.len() > MAX_DEPTH
                || !self.flow.spend(fields.len() + variants.len() + 1)
            {
                return Err(Diagnostic::unsupported(
                    "proof hidden union candidate budget exhausted",
                    expr.span,
                ));
            }
            if !ty.has_borrowed() {
                continue;
            }
            if ty == target || result.pointee() == Some(ty) {
                return Ok(None);
            }
            if let Some(Type::Record {
                primary,
                fields: members,
            }) = Self::origin_record(ty)
            {
                if primary.has_borrowed() {
                    return Ok(None);
                }
                if pending.len() + members.len() > MAX_FIELDS {
                    return Err(Diagnostic::unsupported(
                        "proof hidden union candidate capacity exhausted",
                        expr.span,
                    ));
                }
                for (index, member) in members.iter().enumerate() {
                    let mut fields = fields.clone();
                    fields.push(index);
                    pending.push((&member.ty, fields, variants.clone()));
                }
                continue;
            }
            if let Type::Union(members) = ty {
                if !self.union_carrier_target(ty, expr.span)? {
                    return Ok(None);
                }
                if pending.len() + members.len() > MAX_FIELDS {
                    return Err(Diagnostic::unsupported(
                        "proof hidden union candidate capacity exhausted",
                        expr.span,
                    ));
                }
                for member in members {
                    let mut variants = variants.clone();
                    variants.push((fields.len(), member));
                    pending.push((member, fields.clone(), variants));
                }
                continue;
            }
            let Some(depth) = self.shared_cell_depth(ty, expr)? else {
                return Ok(None);
            };
            let mut terminal = ty;
            for _ in 0..depth {
                terminal = terminal.pointee().unwrap();
            }
            if terminal.has_borrowed() && terminal != target {
                return Ok(None);
            }
            if depth < result_depth {
                continue;
            }
            let layers = depth - result_depth;
            let mut candidate = ty;
            for _ in 0..layers {
                candidate = candidate.pointee().unwrap();
            }
            if !crate::borrow_contract::returns::candidate(result, candidate) {
                continue;
            }
            if !self.origin_carrier(ty, expr.span)? {
                return Ok(None);
            }
            paths.push((
                ShapeKey::new(&fields, &variants, &mut self.flow, expr.span)?,
                layers,
            ));
        }
        Ok(Some(paths))
    }
}

#[cfg(test)]
mod tests;
