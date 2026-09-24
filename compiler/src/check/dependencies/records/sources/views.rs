use super::{Checker, Diagnostic, Expr, Origins, Result, Type};
use crate::check::dependencies::{records::MAX_DEPTH, references::MAX_ROOTS};

impl Checker {
    pub(super) fn record_view_field_origins(
        &mut self,
        view: &Expr,
        path: &[usize],
        depth: usize,
    ) -> Result<Origins> {
        let Type::Reference(target) = &view.ty else {
            return Ok(Origins::default());
        };
        if Self::origin_record(target).is_none() || path.is_empty() {
            return Ok(Origins::default());
        }
        let locations = self.reference_cell_at(view, depth + 1)?;
        if locations.places.len() > MAX_ROOTS {
            return Err(Diagnostic::unsupported(
                "proof record view location capacity exhausted",
                view.span,
            ));
        }
        let mut origins = Origins {
            complete: locations.complete,
            ..Origins::default()
        };
        for (root, prefix) in locations.places {
            let Some(ty) = self.record_view_field_type(view, root, &prefix, path)? else {
                origins.complete = false;
                continue;
            };
            if !Self::origin_reference(ty) {
                origins.complete = false;
                continue;
            }
            let mut fields = prefix;
            fields.extend(path);
            let source = self.cell_origins(root, &fields);
            let work = source.map_or(0, |source| source.roots.len()) + 1;
            if !self.flow.spend(work) {
                return Err(Diagnostic::unsupported(
                    "proof record view origin budget exhausted",
                    view.span,
                ));
            }
            let source = self.cell_origins(root, &fields);
            origins.complete &= source.is_some_and(|source| source.complete);
            if let Some(source) = source {
                origins.roots.extend(&source.roots);
            }
            if origins.roots.len() > MAX_ROOTS {
                return Err(Diagnostic::unsupported(
                    "proof record view origin capacity exhausted",
                    view.span,
                ));
            }
        }
        Ok(origins)
    }

    pub(crate) fn record_view_field_type<'a>(
        &mut self,
        view: &'a Expr,
        root: usize,
        prefix: &[usize],
        path: &[usize],
    ) -> Result<Option<&'a Type>> {
        if prefix.len() + path.len() > MAX_DEPTH || !self.flow.spend(prefix.len() + path.len() + 1)
        {
            return Err(Diagnostic::unsupported(
                "proof record view field budget exhausted",
                view.span,
            ));
        }
        let Type::Reference(target) = &view.ty else {
            return Ok(None);
        };
        let owner = self
            .locals
            .get(root)
            .and_then(|ty| Self::concrete_field_type(ty, prefix));
        if owner != Some(target.as_ref()) {
            return Ok(None);
        }
        Ok(Self::concrete_field_type(target, path))
    }

    pub(super) fn concrete_field_type<'a>(mut ty: &'a Type, path: &[usize]) -> Option<&'a Type> {
        for index in path {
            let Type::Record { fields, .. } = Self::origin_record(ty)? else {
                return None;
            };
            ty = &fields.get(*index)?.ty;
        }
        Some(ty)
    }
}

#[cfg(test)]
mod tests;
