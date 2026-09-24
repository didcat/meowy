use super::{Checker, Diagnostic, Expr, Input, MAX_DEPTH, MAX_FIELDS, Origins, Result, Type};
use crate::check::dependencies::{records::shapes::ShapeKey, references::MAX_ROOTS};

impl Checker {
    pub(super) fn call_union_input_origins(
        &mut self,
        arg: &Expr,
        ty: &Type,
        prefix: &[usize],
        result: &Type,
        depth: usize,
    ) -> Result<Input> {
        let mut pending = vec![(ty, prefix.to_vec(), Vec::new())];
        let mut origins = Origins {
            complete: true,
            ..Origins::default()
        };
        let mut found = false;
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
                    return Ok(Input::Unsupported);
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
                    return Ok(Input::Unsupported);
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
            if !matches!(ty, Type::Reference(_)) || !Self::origin_reference(ty) {
                return Ok(Input::Unsupported);
            }
            if crate::borrow_contract::projections(ty, result, &mut self.flow, arg.span)?.is_empty()
            {
                continue;
            }
            let key = ShapeKey::new(&fields, &variants, &mut self.flow, arg.span)?;
            let source = self.record_shape_source_at(arg, &key, depth + 1)?.origins;
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
}

#[cfg(test)]
mod tests;
