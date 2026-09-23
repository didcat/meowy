use super::{Checker, Diagnostic, Expr, MAX_DEPTH, MAX_FIELDS, Result, Type};

#[derive(Clone)]
pub(crate) struct RecordPath<'a> {
    pub(crate) fields: Vec<usize>,
    pub(crate) variants: Vec<(usize, &'a Type)>,
    pub(crate) supported: bool,
}

impl Checker {
    pub(crate) fn record_paths(&mut self, value: &Expr) -> Result<Vec<Vec<usize>>> {
        self.record_paths_for(value, false)
    }

    pub(crate) fn record_paths_for(
        &mut self,
        value: &Expr,
        carriers: bool,
    ) -> Result<Vec<Vec<usize>>> {
        Ok(self
            .record_origin_paths(value, carriers)?
            .into_iter()
            .filter(|path| path.supported && path.variants.is_empty())
            .map(|path| path.fields)
            .collect())
    }

    pub(crate) fn record_origin_paths<'a>(
        &mut self,
        value: &'a Expr,
        carriers: bool,
    ) -> Result<Vec<RecordPath<'a>>> {
        let path = RecordPath {
            fields: Vec::new(),
            variants: Vec::new(),
            supported: true,
        };
        let mut pending = vec![(path, &value.ty, 0)];
        let mut paths = Vec::new();
        let mut count = 0;
        while let Some((mut path, ty, depth)) = pending.pop() {
            if !self.flow.spend(1) || depth > MAX_DEPTH {
                return Err(Diagnostic::unsupported(
                    "proof record origin budget exhausted",
                    value.span,
                ));
            }
            if !ty.has_reference() {
                continue;
            }
            if let Some(Type::Record { fields, .. }) = Self::origin_record(ty) {
                count += fields.len();
                if count > MAX_FIELDS || depth == MAX_DEPTH || !self.flow.spend(fields.len()) {
                    return Err(Diagnostic::unsupported(
                        "proof record origin budget exhausted",
                        value.span,
                    ));
                }
                for (index, field) in fields.iter().enumerate() {
                    let mut child = path.clone();
                    child.fields.push(index);
                    pending.push((child, &field.ty, depth + 1));
                }
            } else if let Type::Union(members) = ty {
                count += members.len();
                if count > MAX_FIELDS || depth == MAX_DEPTH || !self.flow.spend(members.len()) {
                    return Err(Diagnostic::unsupported(
                        "proof record origin budget exhausted",
                        value.span,
                    ));
                }
                for member in members {
                    if !member.has_reference() {
                        continue;
                    }
                    let mut child = path.clone();
                    child.variants.push((path.fields.len(), member));
                    if Self::origin_record(member).is_some() || matches!(member, Type::Union(_)) {
                        pending.push((child, member, depth + 1));
                    } else {
                        child.supported = false;
                        paths.push(child);
                    }
                }
            } else {
                path.supported = if carriers {
                    self.origin_carrier(ty, value.span)?
                } else {
                    Self::origin_reference(ty)
                };
                paths.push(path);
            }
        }
        Ok(paths)
    }
}

#[cfg(test)]
mod tests;
