use super::{Checker, Leaf, Record, Sources};
use crate::hir::{self, ExprKind, Type};
use std::collections::BTreeMap;

pub(crate) struct Build<'a> {
    pub(crate) target: hir::BlockId,
    pub(crate) fields: &'a [hir::Field],
    pub(crate) record: Record,
    pub(crate) locals: Sources,
    pub(crate) emitted: BTreeMap<String, Option<usize>>,
}

impl Checker {
    pub(crate) fn record_stmts(
        &mut self,
        stmts: &[hir::Stmt],
        depth: usize,
        count: &mut usize,
        build: &mut Build<'_>,
    ) -> Option<bool> {
        if depth >= super::MAX_DEPTH {
            return None;
        }
        let mut types = BTreeMap::new();
        for stmt in stmts {
            if !self.flow.spend(1) {
                return None;
            }
            if let hir::Stmt::Emit {
                field: Some(name),
                value:
                    hir::Expr {
                        kind: ExprKind::Local(id),
                        ..
                    },
                ..
            } = stmt
            {
                if !self.flow.spend(build.fields.len()) {
                    return None;
                }
                types.insert(
                    *id,
                    &build.fields.iter().find(|field| field.name == *name)?.ty,
                );
            }
        }
        for stmt in stmts {
            *count += 1;
            if *count > crate::check::type_values::MAX_WORK || !self.flow.spend(1) {
                return None;
            }
            build.record.input.work = build.record.input.work.saturating_add(1);
            match stmt {
                hir::Stmt::Bind { id, value } if !self.proofs.mutable.contains(id) => {
                    let ty = types
                        .get(id)
                        .copied()
                        .or_else(|| self.locals.get(*id))
                        .unwrap_or(&value.ty)
                        .clone();
                    if matches!(ty, Type::Record { .. }) {
                        let child =
                            self.record_expr(value, &ty, depth + 1, count, &build.locals)?;
                        build.record.input.add(&child.input);
                        build.locals.records.insert(*id, child);
                    } else if ty == Type::Bool {
                        let input = self.predicate_expr(value, depth + 1, count, &build.locals)?;
                        build.record.input.add(&input);
                        if input.error.is_some() {
                            return Some(false);
                        }
                        build.locals.booleans.insert(*id, input);
                    } else {
                        if !matches!(ty, Type::Int { .. } | Type::Never) {
                            return None;
                        }
                        let input = self.input_expr(value, depth + 1, count, &build.locals)?;
                        build.record.input.add(&input);
                        build.locals.integers.insert(*id, input);
                    }
                }
                hir::Stmt::Emit {
                    target,
                    field: Some(name),
                    value,
                    ..
                } if *target == build.target => {
                    if !self.flow.spend(build.fields.len()) {
                        return None;
                    }
                    let index = build.fields.iter().position(|field| field.name == *name)?;
                    let id = match value.kind {
                        ExprKind::Local(id) => Some(id),
                        ExprKind::Field { .. } => None,
                        _ => return None,
                    };
                    if build.emitted.insert(name.clone(), id).is_some() {
                        return None;
                    }
                    if matches!(build.fields[index].ty, Type::Record { .. }) {
                        let child = if let Some(id) = id {
                            self.source_record(id, &build.locals)?.clone()
                        } else {
                            self.record_expr(
                                value,
                                &build.fields[index].ty,
                                depth + 1,
                                count,
                                &build.locals,
                            )?
                        };
                        build.record.input.add(&child.input);
                        for (path, value) in &child.values {
                            let mut target = vec![index];
                            target.extend(path);
                            build.record.values.insert(target, *value);
                        }
                    } else if build.fields[index].ty == Type::Bool {
                        let input = self.predicate_expr(value, depth + 1, count, &build.locals)?;
                        build.record.input.add(&input);
                        build
                            .record
                            .values
                            .insert(vec![index], Leaf::Bool(input.value));
                    } else {
                        let input = self.input_expr(value, depth + 1, count, &build.locals)?;
                        build.record.input.add(&input);
                        build
                            .record
                            .values
                            .insert(vec![index], Leaf::Int(input.value));
                    }
                }
                hir::Stmt::Emit {
                    target,
                    field: None,
                    value,
                    ..
                } if *target == build.target && value.ty == Type::Null => {
                    let ExprKind::Primary(source) = &value.kind else {
                        return None;
                    };
                    let ExprKind::Local(id) = source.kind else {
                        return None;
                    };
                    build
                        .record
                        .input
                        .add(&self.source_record(id, &build.locals)?.input);
                    build.record.input.work = build.record.input.work.saturating_add(2);
                }
                hir::Stmt::If {
                    condition,
                    then,
                    otherwise,
                    ..
                } => {
                    let input = self.predicate_expr(condition, depth + 1, count, &build.locals)?;
                    build.record.input.add(&input);
                    if input.error.is_some() {
                        return Some(false);
                    }
                    let value = input.value?;
                    let locals = build.locals.clone();
                    let branch = if value { then } else { otherwise };
                    if !self.record_stmts(branch, depth + 1, count, build)? {
                        return Some(false);
                    }
                    build.locals = locals;
                }
                hir::Stmt::SlotAlias {
                    id,
                    target,
                    field,
                    mutable: false,
                } if *target == build.target && build.emitted.get(field) == Some(&Some(*id)) => {}
                _ => return None,
            }
        }
        Some(true)
    }
}
