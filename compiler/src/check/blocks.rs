use super::{Checker, Frame, Result, Scope, Slots, Value};
use crate::ast::{self, ExprKind, Span, StmtKind};
use crate::diagnostic::Diagnostic;
use crate::flow::FALSE;
use crate::hir::{self, Type};
use std::collections::BTreeMap;

impl Checker {
    pub(crate) fn block(
        &mut self,
        block: &ast::Block,
        expected: Option<Type>,
        receiver: Option<hir::Expr>,
    ) -> Result<hir::Block> {
        self.block_inner(block, expected, receiver, false)
    }

    pub(crate) fn block_inner(
        &mut self,
        block: &ast::Block,
        expected: Option<Type>,
        receiver: Option<hir::Expr>,
        partial: bool,
    ) -> Result<hir::Block> {
        let mut stmts = self.block_start(block, expected, receiver, partial)?;
        let mut points = Vec::new();
        let mut index = 0;
        while index < block.stmts.len() {
            if matches!(block.stmts[index].kind, StmtKind::Forward { .. }) {
                index = self.forward(&block.stmts, index)?;
                points.push(None);
            } else {
                let (point, checked) = self.checked_stmt(&block.stmts[index])?;
                points.push(Some(point));
                stmts.extend(checked);
                index += 1;
            }
        }
        let body = self.block_end(block, stmts)?;
        self.sequence(
            super::dependencies::SequenceSource::Block(body.id),
            points,
            block.span,
        )?;
        Ok(body)
    }

    pub(crate) fn block_start(
        &mut self,
        block: &ast::Block,
        expected: Option<Type>,
        receiver: Option<hir::Expr>,
        partial: bool,
    ) -> Result<Vec<hir::Stmt>> {
        let id = self.block;
        self.block += 1;
        self.scopes.push(Scope::default());
        if let Some(label) = &block.label {
            self.forget_mutable();
            self.scopes
                .last_mut()
                .expect("scope")
                .labels
                .insert(label.clone(), id);
        }
        self.frames.push(Frame {
            continuation: false,
            id,
            expected,
            leaves: FALSE,
            slots: Slots::new(),
            start: self.writes,
            first: self.proofs.emissions.len(),
            partial,
            owner: self.owner,
        });
        let mut stmts = Vec::new();
        if let Some(value) = receiver {
            self.proofs.dispatches.insert(id);
            let ty = value.ty.clone();
            let local = self.local(ty.clone());
            self.proofs.receivers.insert(local);
            self.places.insert(local);
            let binding = Value::Local {
                id: local,
                ty,
                mutable: false,
                owner: self.owner,
                constant: self.constant(&value),
            };
            self.declare("$", binding, block.span)?;
            stmts.push(hir::Stmt::Bind { id: local, value });
        }
        Ok(stmts)
    }

    pub(crate) fn block_end(
        &mut self,
        block: &ast::Block,
        stmts: Vec<hir::Stmt>,
    ) -> Result<hir::Block> {
        let frame = self.frames.pop().expect("frame");
        let id = frame.id;
        self.reach = self.flow.or(self.reach, frame.leaves);
        let ty = self.block_type(&frame, frame.expected.as_ref(), block.span)?;
        self.block_list_length(&frame, &ty);
        self.validate_aliases(id, &ty)?;
        self.proofs.completions.insert(id, self.reach);
        if self.flow.exceeded() {
            return Err(Diagnostic::unsupported(
                "control-flow proof budget exhausted",
                block.span,
            ));
        }
        if id == self.module.block {
            self.doc_stage(usize::MAX)?;
        }
        self.scopes.pop();
        let body = hir::Block { id, ty, stmts };
        self.track_body(&body, block.span)?;
        Ok(body)
    }

    pub(crate) fn block_type(
        &mut self,
        frame: &Frame,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<Type> {
        if self.reach == FALSE {
            return Ok(Type::Never);
        }
        let mut slots = BTreeMap::new();
        for (name, writes) in &frame.slots {
            let carried = self.proofs.carried.contains_key(&(frame.id, name.clone()));
            let mut types = Vec::new();
            let mut initialized = FALSE;
            let mut mutable = None;
            for slot in writes {
                if !carried && !self.flow.overlap(slot.guard, self.reach) {
                    continue;
                }
                if mutable.is_some_and(|value| value != slot.mutable) {
                    return Err(Self::error(
                        "E206",
                        "result field mutability differs between paths",
                        span,
                    ));
                }
                mutable = Some(slot.mutable);
                types.push(slot.ty.clone());
                initialized = self.flow.or(initialized, slot.guard);
            }
            if types.is_empty() {
                continue;
            }
            if carried {
                initialized = self.reach;
            }
            if !self.flow.implies(self.reach, initialized) {
                types.push(Type::Null);
            }
            slots.insert(
                name.clone(),
                (Type::union(types), initialized, mutable.unwrap_or(false)),
            );
        }
        let constructor =
            expected.is_some_and(Self::record_union) && slots.keys().any(Option::is_some);
        if let Some(expected) = expected.filter(|_| !frame.partial && !constructor) {
            let required: Vec<(Option<String>, Type, bool)> = match expected {
                Type::Record { primary, fields } => {
                    std::iter::once((None, *primary.clone(), false))
                        .chain(fields.iter().map(|field| {
                            (Some(field.name.clone()), field.ty.clone(), field.mutable)
                        }))
                        .collect()
                }
                ty => vec![(None, ty.clone(), false)],
            };
            for (name, ty, mutable) in required {
                let initialized = slots
                    .get(&name)
                    .map(|(_, guard, _)| *guard)
                    .unwrap_or(FALSE);
                if !ty.accepts(&Type::Null) && !self.flow.implies(self.reach, initialized) {
                    return Err(Self::error(
                        "E204",
                        format!(
                            "required result {} is uninitialized on a completing path",
                            name.as_ref()
                                .map(|name| format!("field `{name}`"))
                                .unwrap_or_else(|| "primary".into())
                        ),
                        span,
                    ));
                }
                if slots
                    .get(&name)
                    .is_some_and(|(_, _, value)| *value != mutable)
                {
                    return Err(Self::error(
                        "E206",
                        "result field mutability differs from its declaration",
                        span,
                    ));
                }
                let actual = slots.get(&name).map(|(ty, _, _)| ty).unwrap_or(&Type::Null);
                if !ty.accepts(actual) {
                    return Err(Self::error(
                        "E207",
                        format!("result slot has type {actual:?}, expected {ty:?}"),
                        span,
                    ));
                }
            }
            return Ok(expected.clone());
        }
        let primary = slots
            .remove(&None)
            .map(|(ty, _, _)| ty)
            .unwrap_or(Type::Null);
        let fields: Vec<_> = slots
            .into_iter()
            .filter_map(|(name, (ty, _, mutable))| {
                name.map(|name| hir::Field { name, ty, mutable })
            })
            .collect();
        let actual = if fields.is_empty() {
            primary
        } else {
            Type::Record {
                primary: Box::new(primary),
                fields,
            }
        };
        if constructor && !frame.partial {
            let expected = expected.expect("record union");
            let choices: Vec<_> = expected
                .members()
                .iter()
                .filter(|member| Self::record_fits(member, &actual))
                .collect();
            if choices.len() != 1 {
                return Err(Self::error(
                    "E207",
                    "record constructor does not select one compatible union member",
                    span,
                ));
            }
            return Ok(choices[0].clone());
        }
        Ok(actual)
    }

    pub(crate) fn record_union(ty: &Type) -> bool {
        matches!(ty, Type::Union(members) if members.iter().any(|ty| matches!(ty, Type::Record { .. })))
    }

    pub(crate) fn record_fits(expected: &Type, actual: &Type) -> bool {
        let (
            Type::Record { primary, fields },
            Type::Record {
                primary: value,
                fields: values,
            },
        ) = (expected, actual)
        else {
            return false;
        };
        primary.accepts(value)
            && values.iter().all(|value| {
                fields.iter().any(|field| {
                    field.name == value.name
                        && field.mutable == value.mutable
                        && field.ty.accepts(&value.ty)
                })
            })
            && fields.iter().all(|field| {
                values.iter().any(|value| field.name == value.name) || field.ty.accepts(&Type::Null)
            })
    }

    pub(crate) fn union_slot(ty: &Type, name: Option<&str>, mutable: bool) -> Option<Type> {
        let mut types = Vec::new();
        if name.is_none() {
            types.extend(ty.members().iter().cloned());
        }
        for member in ty.members() {
            if let Type::Record { primary, fields } = member {
                if let Some(name) = name {
                    types.extend(
                        fields
                            .iter()
                            .filter(|field| field.name == name && field.mutable == mutable)
                            .map(|field| field.ty.clone()),
                    );
                } else {
                    types.push(*primary.clone());
                }
            }
        }
        (!types.is_empty()).then(|| Type::union(types))
    }

    pub(crate) fn composed(
        &mut self,
        value: &ast::Expr,
        record: Type,
        expected: Option<&Type>,
    ) -> Result<hir::Expr> {
        match &value.kind {
            ExprKind::Group(value) => self.composed(value, record, expected),
            ExprKind::Block(block) => {
                let block = self.block_inner(block, Some(record), None, true)?;
                Ok(hir::Expr {
                    ty: block.ty.clone(),
                    kind: hir::ExprKind::Block(block),
                    span: value.span,
                })
            }
            ExprKind::DispatchBlock {
                value: receiver,
                block,
            } => {
                let receiver = self.expr(receiver, None)?;
                let block = self.block_inner(block, Some(record), Some(receiver), true)?;
                Ok(hir::Expr {
                    ty: block.ty.clone(),
                    kind: hir::ExprKind::Block(block),
                    span: value.span,
                })
            }
            _ => {
                let value = self.expression(value, expected)?;
                if matches!(value.ty, Type::Record { .. }) {
                    Ok(value)
                } else if let Some(expected) = expected
                    && expected.accepts(&value.ty)
                {
                    Ok(Self::coerce(value, expected.clone()))
                } else {
                    Ok(value)
                }
            }
        }
    }
}
