use super::{Checker, Diagnostic, Expr, MAX_DEPTH, MAX_FIELDS, Result, Type};

impl Checker {
    pub(crate) fn record_paths(&mut self, value: &Expr) -> Result<Vec<Vec<usize>>> {
        self.record_paths_for(value, false)
    }

    pub(crate) fn record_paths_for(
        &mut self,
        value: &Expr,
        carriers: bool,
    ) -> Result<Vec<Vec<usize>>> {
        let mut pending = vec![(Vec::new(), &value.ty)];
        let mut paths = Vec::new();
        let mut count = 0;
        while let Some((path, ty)) = pending.pop() {
            if !ty.has_reference() {
                continue;
            }
            if let Some(Type::Record { fields, .. }) = Self::origin_record(ty) {
                count += fields.len();
                if count > MAX_FIELDS
                    || path.len() >= MAX_DEPTH
                    || !self.flow.spend(fields.len() + 1)
                {
                    return Err(Diagnostic::unsupported(
                        "proof record origin budget exhausted",
                        value.span,
                    ));
                }
                for (index, field) in fields.iter().enumerate() {
                    let mut child = path.clone();
                    child.push(index);
                    pending.push((child, &field.ty));
                }
            } else if if carriers {
                self.origin_carrier(ty, value.span)?
            } else {
                Self::origin_reference(ty)
            } {
                paths.push(path);
            }
        }
        Ok(paths)
    }
}
