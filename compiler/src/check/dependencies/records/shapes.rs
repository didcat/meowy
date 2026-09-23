use super::{Checker, Diagnostic, Expr, MAX_DEPTH, MAX_FIELDS, Result, Type};
use crate::ast::Span;
use crate::check::dependencies::{Cells, Origins, references::MAX_ROOTS};
use crate::flow::Flow;
use std::collections::BTreeMap;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ShapeKey {
    pub(crate) fields: Vec<usize>,
    pub(crate) variants: Vec<(usize, Type)>,
}

#[derive(Clone, Default)]
pub(crate) struct Snapshot {
    pub(crate) origins: Origins,
    pub(crate) cells: Cells,
}

#[derive(Clone, Default)]
pub(crate) struct Shapes {
    pub(super) entries: BTreeMap<ShapeKey, Snapshot>,
}

impl ShapeKey {
    pub(crate) fn new(
        fields: &[usize],
        variants: &[(usize, &Type)],
        flow: &mut Flow,
        span: Span,
    ) -> Result<Self> {
        if fields.len() + variants.len() > MAX_DEPTH
            || fields.iter().any(|index| *index >= MAX_FIELDS)
            || variants.iter().any(|(at, _)| *at > fields.len())
            || variants.windows(2).any(|pair| pair[0].0 > pair[1].0)
            || !flow.spend(fields.len() + variants.len() + 1)
        {
            return Err(Diagnostic::unsupported(
                "proof record shape key budget exhausted",
                span,
            ));
        }
        let mut nodes = 0;
        for (_, ty) in variants {
            nodes += crate::borrow_contract::type_weight(ty, flow, span)?;
            if nodes > MAX_FIELDS * MAX_DEPTH {
                return Err(Diagnostic::unsupported(
                    "proof record shape type budget exhausted",
                    span,
                ));
            }
        }
        Ok(Self {
            fields: fields.to_vec(),
            variants: variants
                .iter()
                .map(|(at, ty)| (*at, (*ty).clone()))
                .collect(),
        })
    }
}

impl Shapes {
    pub(crate) fn snapshots(&self, path: &[usize]) -> impl Iterator<Item = &Snapshot> {
        self.entries
            .iter()
            .filter(move |(key, _)| key.fields.starts_with(path))
            .map(|(_, value)| value)
    }

    pub(crate) fn get(&self, key: &ShapeKey) -> Option<&Snapshot> {
        self.entries.get(key)
    }

    pub(crate) fn insert(
        &mut self,
        key: ShapeKey,
        value: Snapshot,
        flow: &mut Flow,
        span: Span,
    ) -> Result<()> {
        let work = value.origins.roots.len()
            + value
                .cells
                .places
                .iter()
                .map(|(_, path)| path.len() + 1)
                .sum::<usize>()
            + 1;
        if (self.entries.len() == MAX_FIELDS && self.get(&key).is_none())
            || value.origins.roots.len() > MAX_ROOTS
            || value.cells.places.len() > MAX_ROOTS
            || value
                .cells
                .places
                .iter()
                .any(|(_, path)| path.len() > MAX_DEPTH)
            || !flow.spend(work)
        {
            return Err(Diagnostic::unsupported(
                "proof record shape snapshot budget exhausted",
                span,
            ));
        }
        self.entries.insert(key, value);
        Ok(())
    }
}

impl Checker {
    pub(crate) fn shaped(&self, id: usize) -> Option<&Shapes> {
        self.record_shapes.get(&self.origin_id(id))
    }

    pub(crate) fn track_record_shapes(&mut self, id: usize, value: &Expr) -> Result<()> {
        self.build_record_shapes(id, value, false)
    }

    pub(crate) fn capture_record_shapes(&mut self, id: usize, value: &Expr) -> Result<()> {
        if value.ty.has_mutable_fields() {
            return Ok(());
        }
        self.build_record_shapes(id, value, true)
    }

    pub(super) fn build_record_shapes(
        &mut self,
        id: usize,
        value: &Expr,
        capture: bool,
    ) -> Result<()> {
        let shapes = self.record_shape_values(value, capture)?;
        if !shapes.entries.is_empty() {
            self.record_shapes.insert(id, shapes);
        }
        Ok(())
    }

    pub(super) fn record_shape_values(&mut self, value: &Expr, capture: bool) -> Result<Shapes> {
        let mut shapes = Shapes::default();
        for carriers in [false, true] {
            for path in self.record_origin_paths(value, carriers)? {
                if path.variants.is_empty() {
                    continue;
                }
                let key = ShapeKey::new(&path.fields, &path.variants, &mut self.flow, value.span)?;
                let snapshot = if capture {
                    self.record_shape_source(value, &key)?
                } else {
                    Snapshot::default()
                };
                shapes.insert(key, snapshot, &mut self.flow, value.span)?;
            }
        }
        Ok(shapes)
    }
}

#[cfg(test)]
mod tests;

mod reads;

#[cfg(test)]
mod dependencies;

mod producers;

mod aliases;
