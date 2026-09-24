use super::{Checker, Diagnostic, Expr, ExprKind, Origins, Result, Type};

pub(crate) enum RecordSource {
    Unknown,
    Empty,
    Direct(crate::hir::Place),
    Alternatives(Vec<crate::hir::Place>),
}

impl Checker {
    pub(crate) fn record_source_path<'a>(
        &mut self,
        value: &'a Expr,
        path: &[usize],
    ) -> Result<(&'a Expr, Vec<usize>)> {
        if path.len() > super::MAX_DEPTH || !self.flow.spend(path.len() + 1) {
            return Err(Diagnostic::unsupported(
                "proof record source path budget exhausted",
                value.span,
            ));
        }
        let mut base = value;
        let mut path = path.iter().rev().copied().collect::<Vec<_>>();
        let mut steps = path.len();
        loop {
            let inner = match &base.kind {
                ExprKind::Field {
                    value: inner,
                    index,
                } => {
                    if path.len() == super::MAX_DEPTH {
                        return Err(Diagnostic::unsupported(
                            "proof record source path budget exhausted",
                            value.span,
                        ));
                    }
                    path.push(*index);
                    inner
                }
                ExprKind::Coerce { value: inner }
                    if Self::origin_record(&base.ty).is_some()
                        && Self::origin_record(&base.ty) == Self::origin_record(&inner.ty) =>
                {
                    inner
                }
                _ => break,
            };
            if steps == super::MAX_DEPTH || !self.flow.spend(1) {
                return Err(Diagnostic::unsupported(
                    "proof record source path budget exhausted",
                    value.span,
                ));
            }
            steps += 1;
            base = inner;
        }
        path.reverse();
        Ok((base, path))
    }

    pub(crate) fn record_source_locations(
        &mut self,
        value: &Expr,
        path: &[usize],
    ) -> Result<RecordSource> {
        let mut value = value;
        while let ExprKind::Coerce { value: inner } = &value.kind {
            let Some(record) = Self::origin_record(&value.ty) else {
                return Ok(RecordSource::Unknown);
            };
            if inner.ty == Type::Null {
                return Ok(RecordSource::Empty);
            }
            if Self::origin_record(&inner.ty) != Some(record) {
                return Ok(RecordSource::Unknown);
            }
            value = inner;
        }
        if matches!(
            value.kind,
            ExprKind::Local(_) | ExprKind::Field { .. } | ExprKind::Deref(_)
        ) {
            return Ok(Self::record_location(value, path)
                .map_or(RecordSource::Unknown, RecordSource::Direct));
        }
        let ExprKind::Block(block) = &value.kind else {
            return Ok(RecordSource::Unknown);
        };
        let Some(Type::Record { fields, .. }) = Self::origin_record(&value.ty) else {
            return Ok(RecordSource::Unknown);
        };
        let Some((index, tail)) = path.split_first() else {
            return Ok(RecordSource::Unknown);
        };
        if !self.flow.spend(self.proofs.aliases.len()) {
            return Err(Diagnostic::unsupported(
                "proof record origin budget exhausted",
                value.span,
            ));
        }
        let mut sources = Vec::new();
        for (id, alias) in &self.proofs.aliases {
            if alias.target != block.id || alias.field != fields[*index].name {
                continue;
            }
            sources.push(crate::hir::Place {
                root: if tail.is_empty() { alias.root } else { *id },
                fields: tail.to_vec(),
            });
        }
        if let Some(compositions) = self.record_compositions.get(&block.id) {
            if !self.flow.spend(compositions.len()) {
                return Err(Diagnostic::unsupported(
                    "proof record origin budget exhausted",
                    value.span,
                ));
            }
            for id in compositions {
                let Type::Record {
                    fields: source_fields,
                    ..
                } = &self.locals[*id]
                else {
                    unreachable!()
                };
                if !self.flow.spend(source_fields.len()) {
                    return Err(Diagnostic::unsupported(
                        "proof record origin budget exhausted",
                        value.span,
                    ));
                }
                if let Some(source_index) = source_fields
                    .iter()
                    .position(|field| field.name == fields[*index].name)
                {
                    let mut source_path = vec![source_index];
                    source_path.extend(tail);
                    sources.push(crate::hir::Place {
                        root: *id,
                        fields: source_path,
                    });
                }
            }
        }
        Ok(if sources.is_empty() {
            RecordSource::Unknown
        } else {
            RecordSource::Alternatives(sources)
        })
    }

    pub(crate) fn record_source_origins(
        &mut self,
        value: &Expr,
        path: &[usize],
    ) -> Result<Origins> {
        self.record_source_origins_at(value, path, 0)
    }

    pub(crate) fn record_source_origins_at(
        &mut self,
        value: &Expr,
        path: &[usize],
        depth: usize,
    ) -> Result<Origins> {
        if depth > crate::check::dependencies::calls::MAX_DEPTH {
            return Err(Diagnostic::unsupported(
                "proof record origin call depth exhausted",
                value.span,
            ));
        }
        let (base, fields) = self.record_source_path(value, path)?;
        if let Some(snapshot) = self.record_shape_snapshot(value, path)? {
            return Ok(snapshot.origins);
        }
        if let ExprKind::Call { args, .. } = &base.kind {
            return self.record_call_field_origins(base, args, &fields, depth);
        }
        let sources = match self.record_source_locations(value, path)? {
            RecordSource::Unknown => {
                if let ExprKind::Deref(view) = &base.kind {
                    return self.record_view_field_origins(view, &fields, depth);
                }
                return Ok(Origins::default());
            }
            RecordSource::Empty => {
                return Ok(Origins {
                    roots: Default::default(),
                    complete: true,
                });
            }
            RecordSource::Direct(place) => {
                return Ok(self
                    .record_pointees
                    .get(&place.root)
                    .and_then(|fields| fields.get(&place.fields))
                    .cloned()
                    .unwrap_or_default());
            }
            RecordSource::Alternatives(sources) => sources,
        };
        let mut origins = Origins::default();
        let mut found = false;
        for place in sources {
            let work = self
                .cell_origins(place.root, &place.fields)
                .map_or(0, |source| source.roots.len())
                + 1;
            if !self.flow.spend(work) {
                return Err(Diagnostic::unsupported(
                    "proof record origin budget exhausted",
                    value.span,
                ));
            }
            let source = self.cell_origins(place.root, &place.fields);
            let complete = source.is_some_and(|source| source.complete);
            origins.complete = if found {
                origins.complete && complete
            } else {
                complete
            };
            found = true;
            if let Some(source) = source {
                origins.roots.extend(&source.roots);
            }
            if origins.roots.len() > super::super::references::MAX_ROOTS {
                return Err(Diagnostic::unsupported(
                    "proof record origin capacity exhausted",
                    value.span,
                ));
            }
        }
        Ok(origins)
    }
}

#[cfg(test)]
mod tests;

mod calls;

#[cfg(test)]
mod coercions;

mod views;
