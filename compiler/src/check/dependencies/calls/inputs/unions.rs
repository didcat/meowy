use super::{Checker, Diagnostic, Expr, Input, MAX_DEPTH, MAX_FIELDS, Origins, Result, Type};
use crate::check::dependencies::{Cells, records::shapes::ShapeKey, references::MAX_ROOTS};

pub(super) struct UnionLeaf<'a> {
    pub(super) key: ShapeKey,
    pub(super) view: &'a Type,
    pub(super) layers: usize,
}

pub(super) enum UnionSource<'a> {
    Value(usize),
    Stored(&'a Cells),
}

impl Checker {
    pub(super) fn call_union_input_origins(
        &mut self,
        arg: &Expr,
        ty: &Type,
        prefix: &[usize],
        result: &Type,
        depth: usize,
    ) -> Result<Input> {
        let Some(leaves) = self.union_input_leaves(ty, prefix, result, arg)? else {
            return Ok(Input::Unsupported);
        };
        self.resolve_union_origins(arg, result, leaves, UnionSource::Value(depth), 0)
    }

    pub(super) fn call_union_location_origins(
        &mut self,
        cells: &Cells,
        arg: &Expr,
        ty: &Type,
        prefix: &[usize],
        result: &Type,
        level: usize,
    ) -> Result<Input> {
        if level > MAX_DEPTH {
            return Err(Diagnostic::unsupported(
                "proof union input depth exhausted",
                arg.span,
            ));
        }
        let Some(leaves) = self.union_input_leaves(ty, prefix, result, arg)? else {
            return Ok(Input::Unsupported);
        };
        self.resolve_union_origins(arg, result, leaves, UnionSource::Stored(cells), level)
    }

    pub(super) fn resolve_union_origins(
        &mut self,
        arg: &Expr,
        result: &Type,
        leaves: Vec<UnionLeaf<'_>>,
        source: UnionSource<'_>,
        level: usize,
    ) -> Result<Input> {
        let mut origins = Origins {
            complete: match source {
                UnionSource::Value(_) => true,
                UnionSource::Stored(cells) => cells.complete,
            },
            ..Origins::default()
        };
        let mut found = false;
        for leaf in leaves {
            let next = level + leaf.key.fields.len() + leaf.key.variants.len();
            if next > MAX_DEPTH {
                return Err(Diagnostic::unsupported(
                    "proof union input depth exhausted",
                    arg.span,
                ));
            }
            let snapshot = match source {
                UnionSource::Value(depth) => {
                    self.record_shape_source_at(arg, &leaf.key, depth + 1)?
                }
                UnionSource::Stored(cells) => {
                    self.location_shape_source(cells, &[], &leaf.key, arg.span)?
                }
            };
            let source = if leaf.view.pointee().is_some_and(Type::has_borrowed) {
                let target = leaf.view.pointee().unwrap();
                let found = if Self::origin_record(target).is_some() {
                    self.call_record_cell_origins(
                        snapshot.cells,
                        arg,
                        leaf.view,
                        result,
                        leaf.layers,
                        next,
                    )?
                } else {
                    let mut cells = snapshot.cells;
                    for _ in 0..leaf.layers {
                        cells = self.expand_reference_cells(cells, arg)?;
                    }
                    self.call_union_location_origins(&cells, arg, target, &[], result, next + 1)?
                };
                match found {
                    Input::Unsupported => return Ok(Input::Unsupported),
                    Input::Absent => continue,
                    Input::Known(source) => source,
                }
            } else if leaf.layers == 0 {
                snapshot.origins
            } else {
                self.call_stored_origins(snapshot.cells, arg, leaf.layers - 1)?
            };
            if !self.flow.spend(source.roots.len() + 1) {
                return Err(Diagnostic::unsupported(
                    "proof union input origin budget exhausted",
                    arg.span,
                ));
            }
            found = true;
            origins.complete &= source.complete;
            origins.roots.extend(source.roots);
            if origins.roots.len() > MAX_ROOTS {
                return Err(Diagnostic::unsupported(
                    "proof union input origin capacity exhausted",
                    arg.span,
                ));
            }
        }
        Ok(if found {
            Input::Known(origins)
        } else {
            Input::Absent
        })
    }

    pub(super) fn call_origin_view<'a>(
        &mut self,
        ty: &'a Type,
        arg: &Expr,
    ) -> Result<Option<(&'a Type, usize)>> {
        if let Some(view) = self.call_shared_view(ty, arg)? {
            return Ok(Some(view));
        }
        let Some(depth) = self.shared_cell_depth(ty, arg)? else {
            return Ok(None);
        };
        let mut view = ty;
        for _ in 1..depth {
            view = view.pointee().unwrap();
        }
        if !self.union_carrier_target(view.pointee().unwrap(), arg.span)? {
            return Ok(None);
        }
        Ok(Some((view, depth - 1)))
    }

    pub(super) fn union_input_leaves<'a>(
        &mut self,
        ty: &'a Type,
        prefix: &[usize],
        result: &Type,
        arg: &Expr,
    ) -> Result<Option<Vec<UnionLeaf<'a>>>> {
        let mut pending = vec![(ty, prefix.to_vec(), Vec::new())];
        let mut leaves = Vec::new();
        let mut visits = 0;
        while let Some((ty, fields, variants)) = pending.pop() {
            visits += 1;
            if visits > MAX_FIELDS
                || fields.len() + variants.len() > MAX_DEPTH
                || !self.flow.spend(fields.len() + variants.len() + 1)
            {
                return Err(Diagnostic::unsupported(
                    "proof union input origin budget exhausted",
                    arg.span,
                ));
            }
            if !ty.has_borrowed() {
                continue;
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
                        "proof union input capacity exhausted",
                        arg.span,
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
                if !self.union_carrier_target(ty, arg.span)? {
                    return Ok(None);
                }
                if pending.len() + members.len() > MAX_FIELDS {
                    return Err(Diagnostic::unsupported(
                        "proof union input capacity exhausted",
                        arg.span,
                    ));
                }
                for member in members {
                    let mut variants = variants.clone();
                    variants.push((fields.len(), member));
                    pending.push((member, fields.clone(), variants));
                }
                continue;
            }
            let Some((view, layers)) = self.call_origin_view(ty, arg)? else {
                return Ok(None);
            };
            let nested = view.pointee().is_some_and(Type::has_borrowed);
            if !nested && !Self::origin_reference(view) {
                return Ok(None);
            }
            if !nested
                && crate::borrow_contract::projections(view, result, &mut self.flow, arg.span)?
                    .is_empty()
            {
                continue;
            }
            leaves.push(UnionLeaf {
                key: ShapeKey::new(&fields, &variants, &mut self.flow, arg.span)?,
                view,
                layers,
            });
        }
        Ok(Some(leaves))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod carriers;

#[cfg(test)]
mod records;

#[cfg(test)]
mod borrowed_unions;
