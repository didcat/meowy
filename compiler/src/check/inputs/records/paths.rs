use super::{Checker, Input, MAX_DEPTH, Record, Sources};
use crate::check::exports;
use crate::hir::{self, ExprKind, Type};

impl Record {
    pub(crate) fn project(&self, path: &[usize]) -> Option<Self> {
        let values = self
            .values
            .iter()
            .filter_map(|(key, value)| {
                let suffix = key.strip_prefix(path)?;
                (!suffix.is_empty()).then(|| (suffix.to_vec(), *value))
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        if values.is_empty() {
            return None;
        }
        let mut input = self.input.clone();
        input.work = input.work.saturating_add(path.len());
        Some(Self { input, values })
    }
}

impl Checker {
    pub(crate) fn input_path(&mut self, id: usize, path: &[usize]) -> Option<exports::Input> {
        let Some(module) = self.exports.get(&id) else {
            return Some(exports::Input {
                id,
                path: path.to_vec(),
                work: 0,
            });
        };
        let (index, tail) = path.split_first()?;
        let Type::Record { fields, .. } = self.locals.get(id)? else {
            return None;
        };
        let field = fields.get(*index)?;
        if !self.flow.spend(field.name.len() + module.inputs.len() + 1) {
            return None;
        }
        let source = module.inputs.get(&field.name)?;
        if source.path.len().saturating_add(tail.len()) > MAX_DEPTH {
            return None;
        }
        let mut input = source.clone();
        input.path.extend_from_slice(tail);
        input.work = input.work.saturating_add(1);
        Some(input)
    }

    pub(crate) fn boolean_field_input(
        &mut self,
        id: usize,
        path: &[usize],
        locals: &Sources,
    ) -> Option<Input<bool>> {
        let source = self.input_path(id, path)?;
        let mut input = self
            .source_record(source.id, locals)?
            .boolean(&source.path)?;
        input.work = input.work.saturating_add(source.work);
        Some(input)
    }

    pub(crate) fn field_input(
        &mut self,
        id: usize,
        path: &[usize],
        locals: &Sources,
    ) -> Option<Input> {
        let source = self.input_path(id, path)?;
        let mut input = if source.path.is_empty() {
            locals
                .integers
                .get(&source.id)
                .or_else(|| self.inputs.get(&source.id))?
                .clone()
        } else {
            self.source_record(source.id, locals)?.field(&source.path)?
        };
        input.work = input.work.saturating_add(source.work);
        Some(input)
    }

    pub(crate) fn record_path(&self, expr: &hir::Expr) -> Option<(usize, Vec<usize>)> {
        let mut root = expr;
        let mut path = Vec::new();
        while let ExprKind::Field { value, index } = &root.kind {
            if path.len() == MAX_DEPTH + 1 {
                return None;
            }
            path.push(*index);
            root = value;
        }
        let ExprKind::Local(id) = root.kind else {
            return None;
        };
        if path.len() > MAX_DEPTH && !self.exports.contains_key(&id) {
            return None;
        }
        path.reverse();
        Some((id, path))
    }
}
