use super::{Checker, Diagnostic, Expr, Input, MAX_DEPTH, MAX_FIELDS, Origins, Result, Type};
use crate::check::dependencies::{records::shapes::ShapeKey, references::MAX_ROOTS};

pub(super) struct UnionLeaf<'a> {
    pub(super) key: ShapeKey,
    pub(super) view: &'a Type,
    pub(super) layers: usize,
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
        let mut origins = Origins {
            complete: true,
            ..Origins::default()
        };
        let mut found = false;
        for leaf in leaves {
            let snapshot = self.record_shape_source_at(arg, &leaf.key, depth + 1)?;
            let source = if leaf.view.pointee().is_some_and(Type::has_borrowed) {
                match self.call_record_cell_origins(
                    snapshot.cells,
                    arg,
                    leaf.view,
                    result,
                    leaf.layers,
                    leaf.key.fields.len() + leaf.key.variants.len(),
                )? {
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
            let Some((view, layers)) = self.call_shared_view(ty, arg)? else {
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
