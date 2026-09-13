mod build;
mod paths;

use super::{Checker, Input, Sources};
use crate::hir::{self, ExprKind, Type};
use std::collections::BTreeMap;

pub(crate) const MAX_FIELDS: usize = 256;
pub(crate) const MAX_DEPTH: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Leaf {
    Int(Option<i128>),
    Bool(Option<bool>),
}

#[derive(Clone, Debug)]
pub(crate) struct Record {
    pub(crate) input: Input,
    pub(crate) values: BTreeMap<Vec<usize>, Leaf>,
}

impl Record {
    pub(crate) fn failed(&mut self, fields: &[hir::Field]) -> Option<()> {
        self.input.error.as_ref()?;
        let mut pending = fields
            .iter()
            .enumerate()
            .map(|(index, field)| (vec![index], &field.ty))
            .collect::<Vec<_>>();
        while let Some((path, ty)) = pending.pop() {
            match ty {
                Type::Int { .. } => {
                    self.values.insert(path, Leaf::Int(None));
                }
                Type::Bool => {
                    self.values.insert(path, Leaf::Bool(None));
                }
                Type::Record { fields, .. } => {
                    for (index, field) in fields.iter().enumerate() {
                        let mut path = path.clone();
                        path.push(index);
                        pending.push((path, &field.ty));
                    }
                }
                _ => return None,
            }
        }
        Some(())
    }

    pub(crate) fn boolean(&self, path: &[usize]) -> Option<Input<bool>> {
        let Leaf::Bool(value) = *self.values.get(path)? else {
            return None;
        };
        Some(Input {
            work: self.input.work.saturating_add(path.len()),
            error: self.input.error.clone(),
            value: if self.input.error.is_none() {
                value
            } else {
                None
            },
        })
    }

    pub(crate) fn field(&self, path: &[usize]) -> Option<Input> {
        let Leaf::Int(value) = *self.values.get(path)? else {
            return None;
        };
        let mut input = self.input.clone();
        input.work = input.work.saturating_add(path.len());
        input.value = if input.error.is_none() { value } else { None };
        Some(input)
    }
}

impl Checker {
    pub(crate) fn source_record<'a>(
        &'a self,
        id: usize,
        locals: &'a Sources,
    ) -> Option<&'a Record> {
        locals
            .records
            .get(&id)
            .or_else(|| self.record_inputs.get(&id))
    }

    pub(crate) fn record_shape(&mut self, ty: &Type) -> bool {
        let mut pending = vec![(ty, 1)];
        let mut count = 0;
        while let Some((ty, depth)) = pending.pop() {
            if !self.flow.spend(1) {
                return false;
            }
            match ty {
                Type::Int { .. } | Type::Bool => {}
                Type::Record { primary, fields } => {
                    count += fields.len();
                    if **primary != Type::Null
                        || fields.is_empty()
                        || count > MAX_FIELDS
                        || depth > MAX_DEPTH
                    {
                        return false;
                    }
                    for field in fields {
                        if field.mutable {
                            return false;
                        }
                        pending.push((&field.ty, depth + 1));
                    }
                }
                _ => return false,
            }
        }
        true
    }

    pub(crate) fn record_input(&mut self, expr: &hir::Expr, ty: &Type) -> Option<Record> {
        if !matches!(ty, Type::Record { .. }) {
            return None;
        }
        self.record_expr(expr, ty, 0, &mut 0, &Sources::default())
    }

    pub(crate) fn record_expr(
        &mut self,
        expr: &hir::Expr,
        ty: &Type,
        depth: usize,
        count: &mut usize,
        locals: &Sources,
    ) -> Option<Record> {
        *count += 1;
        if *count > super::super::type_values::MAX_WORK
            || depth >= MAX_DEPTH
            || !self.flow.spend(1)
            || !self.record_shape(ty)
        {
            return None;
        }
        let Type::Record { fields, .. } = ty else {
            return None;
        };
        if let ExprKind::Local(id) = &expr.kind {
            if self.locals.get(*id) != Some(ty) {
                return None;
            }
            let mut record = self.source_record(*id, locals)?.clone();
            record.input.work = record.input.work.saturating_add(1);
            return Some(record);
        }
        if matches!(expr.kind, ExprKind::Field { .. }) {
            let (id, path) = self.record_path(expr)?;
            let source = self.input_path(id, &path)?;
            let mut record = self
                .source_record(source.id, locals)?
                .project(&source.path)?;
            record.input.work = record.input.work.saturating_add(source.work);
            return Some(record);
        }
        let ExprKind::Block(block) = &expr.kind else {
            return None;
        };
        let mut build = build::Build {
            target: block.id,
            fields,
            record: Record {
                input: Input {
                    work: 1,
                    error: None,
                    value: None,
                },
                values: BTreeMap::new(),
            },
            locals: locals.clone(),
            emitted: BTreeMap::new(),
        };
        if !self.record_stmts(&block.stmts, depth, count, &mut build)? {
            build.record.failed(fields)?;
            return Some(build.record);
        }
        if build.emitted.len() != fields.len()
            || depth == 0 && expr.ty == Type::Never && build.record.input.error.is_none()
        {
            return None;
        }
        Some(build.record)
    }
}

#[cfg(test)]
mod tests;
