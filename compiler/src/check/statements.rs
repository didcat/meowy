use super::{Checker, Result, Scope, Slot, Value};
use crate::ast::{self, ExprKind, Span, StmtKind};
use crate::diagnostic::Diagnostic;
use crate::flow::FALSE;
use crate::hir::{self, Type};

impl Checker {
    pub(crate) fn stmt(&mut self, stmt: &ast::Stmt) -> Result<Vec<hir::Stmt>> {
        if self.statements >= 65_536 || !self.flow.spend(1) {
            return Err(Diagnostic::unsupported(
                "statement lifetime budget exhausted",
                stmt.span,
            ));
        }
        let id = self.statements;
        self.statements += 1;
        self.statement.push((id, false));
        let result = self.stmt_inner(stmt);
        let (_, used) = self.statement.pop().expect("statement lifetime");
        let stmts = result?;
        self.doc_stage(stmt.span.start)?;
        if used {
            Ok(vec![hir::Stmt::Statement { id, stmts }])
        } else {
            Ok(stmts)
        }
    }

    pub(crate) fn stmt_inner(&mut self, stmt: &ast::Stmt) -> Result<Vec<hir::Stmt>> {
        if self.pending_statement(stmt)? {
            return Ok(Vec::new());
        }
        match &stmt.kind {
            StmtKind::Bind {
                name,
                ty,
                mutable,
                value,
            } => {
                if let ExprKind::Function { params, body } = &value.kind {
                    if *mutable {
                        return Err(Diagnostic::unsupported(
                            "mutable function bindings",
                            stmt.span,
                        ));
                    }
                    self.declare_function(name, ty.as_ref(), params, body, stmt.span)?;
                    return Ok(Vec::new());
                }
                if let Some(annotation) = ty
                    && self.meta_annotation(annotation)?
                {
                    if *mutable {
                        return Err(Diagnostic::unsupported(
                            "mutable type-value bindings",
                            stmt.span,
                        ));
                    }
                    let value = self.meta_binding(value, annotation)?;
                    self.declare(name, value, stmt.span)?;
                    return Ok(Vec::new());
                }
                if let Some(symbol) = self.binding_symbol(value)?
                    && !matches!(
                        symbol,
                        Value::Local { .. }
                            | Value::Constant(_)
                            | Value::Foundation(crate::foundation::Item::Heap)
                    )
                    && !(matches!(symbol, Value::Static { .. }) && (*mutable || ty.is_some()))
                {
                    if *mutable || ty.is_some() {
                        return Err(Diagnostic::unsupported(
                            "annotated or mutable compile-time identity bindings",
                            stmt.span,
                        ));
                    }
                    self.declare(name, symbol, stmt.span)?;
                    return Ok(Vec::new());
                }
                let expected = ty.as_ref().map(|ty| self.construct_type(ty)).transpose()?;
                let (value, exports) = if name.starts_with('\0') {
                    let (value, module) = self.module_value(value, expected.as_ref())?;
                    (value, Some(module))
                } else {
                    (self.expr(value, expected.as_ref())?, None)
                };
                let ty = expected.unwrap_or_else(|| value.ty.clone());
                if name.starts_with('\0')
                    && name != "\0module0"
                    && (ty.has_mutable_fields()
                        || (ty != Type::Null
                            && ty != Type::Never
                            && !crate::borrow::carried::eligible(&ty, &mut self.flow, stmt.span)?))
                {
                    return Err(Diagnostic::unsupported(
                        "file-module exports outside immutable reference-free values",
                        stmt.span,
                    ));
                }
                if *mutable
                    && ty.has_reference()
                    && !matches!(ty, Type::Reference(_) | Type::Exclusive(_))
                    && !ty.fixed_borrowed_value()
                {
                    return Err(Diagnostic::unsupported(
                        "mutable reference bindings",
                        stmt.span,
                    ));
                }
                let id = self.local(ty.clone());
                self.track_reference(id, &value, false)?;
                self.track_reference_cell(id, &value, false)?;
                self.track_record_references(id, &value, false)?;
                if self.control || self.derived_expr(&value) {
                    self.mark_derived(id);
                }
                if let Some(exports) = exports {
                    self.exports.insert(id, exports);
                }
                if !*mutable && let Some(fact) = self.list_fact(&value) {
                    self.lengths.insert(id, fact);
                }
                if *mutable {
                    self.proofs.mutable.insert(id);
                }
                self.places.insert(id);
                let constant = if *mutable || self.derived.contains(&id) {
                    None
                } else {
                    self.constant(&value)
                };
                if !*mutable && !name.starts_with('\0') {
                    if let Some(mut input) = self.integer_input(&value, &ty) {
                        input.derived |= self.derived.contains(&id);
                        self.inputs.insert(id, input);
                    }
                    if let Some(mut input) = self.boolean_input(&value, &ty) {
                        input.derived |= self.derived.contains(&id);
                        self.bool_inputs.insert(id, input);
                    }
                    if let Some(mut input) = self.record_input(&value, &ty) {
                        input.input.derived |= self.derived.contains(&id);
                        self.record_inputs.insert(id, input);
                    }
                }
                self.declare(
                    name,
                    Value::Local {
                        id,
                        ty,
                        mutable: *mutable,
                        owner: self.owner,
                        constant,
                    },
                    stmt.span,
                )?;
                Ok(vec![hir::Stmt::Bind { id, value }])
            }
            StmtKind::TypeAlias { name, ty, exported } => {
                self.declare_type(name, ty, *exported, stmt.span)?;
                Ok(Vec::new())
            }
            StmtKind::Assign { target, value } => {
                let mut form = target;
                while let ExprKind::Group(value) = &form.kind {
                    form = value;
                }
                if let ExprKind::Unary { op, value: pointer } = &form.kind
                    && op == "*"
                {
                    let target = self.expr(pointer, None)?;
                    if target.ty.pointee().is_some_and(|ty| {
                        !matches!(ty, Type::Bool | Type::Int { .. } | Type::Float { .. })
                    }) {
                        return Err(Diagnostic::unsupported(
                            "indirect stores outside scalar storage",
                            form.span,
                        ));
                    }
                    let Type::Exclusive(ty) = &target.ty else {
                        return Err(Self::error(
                            "E305",
                            "indirect mutation requires exclusive access",
                            form.span,
                        ));
                    };
                    let value = self.expr(value, Some(ty))?;
                    if self.control || self.derived_expr(&target) || self.derived_expr(&value) {
                        let origins = self.reference_origins(&target)?;
                        if !origins.complete {
                            return Err(Diagnostic::unsupported(
                                "proof dependency tracking for indirect store origins",
                                form.span,
                            ));
                        }
                        for root in origins.roots {
                            self.mark_derived(root);
                        }
                    }
                    self.forget_mutable();
                    return Ok(vec![hir::Stmt::Store {
                        target,
                        value,
                        span: form.span,
                    }]);
                }
                if matches!(form.kind, ExprKind::Index { .. } | ExprKind::Field { .. }) {
                    return Ok(vec![self.write_path(target, value)?]);
                }
                let ExprKind::Name(name) = &target.kind else {
                    return Err(Diagnostic::unsupported(
                        "assignment through fields or references",
                        target.span,
                    ));
                };
                let Value::Local {
                    id, ty, mutable, ..
                } = self.value(name, target.span)?
                else {
                    return Err(Self::error(
                        "E305",
                        "assignment requires a mutable local binding",
                        target.span,
                    ));
                };
                if !mutable {
                    return Err(Self::error(
                        "E305",
                        format!("binding `{name}` is immutable"),
                        target.span,
                    ));
                }
                if !self.places.contains(&id) && !self.proofs.aliases.contains_key(&id) {
                    return Err(Diagnostic::unsupported(
                        "assignment to emitted result bindings",
                        target.span,
                    ));
                }
                let value = self.expr(value, Some(&ty))?;
                self.track_reference(id, &value, true)?;
                self.track_reference_cell(id, &value, true)?;
                self.track_record_references(id, &value, true)?;
                if self.control || self.derived_expr(&value) {
                    self.mark_derived(id);
                }
                self.forget(id);
                Ok(vec![hir::Stmt::Assign { id, value }])
            }
            StmtKind::Emit {
                label,
                name,
                ty,
                mutable,
                value,
            } => self.emit(
                label.as_deref(),
                name.as_deref(),
                ty.as_ref(),
                *mutable,
                value,
                stmt.span,
            ),
            StmtKind::Match { arms } => {
                let mut stmts = Vec::new();
                for (condition, body) in arms {
                    let Some(condition) = condition else {
                        return Err(Diagnostic::unsupported("matcher fallback arms", stmt.span));
                    };
                    let condition = self.expr(condition, None)?;
                    if condition.ty != Type::Bool {
                        return Err(Self::error(
                            "E215",
                            format!("matcher requires boolean, found {:?}", condition.ty),
                            condition.span,
                        ));
                    }
                    let guard = self.guard(&condition);
                    let absent = self.flow.not(guard);
                    let skipped = self.flow.and(self.reach, absent);
                    self.reach = self.flow.and(self.reach, guard);
                    let control = self.control;
                    self.control |= self.derived_expr(&condition);
                    let depth = self.scopes.len();
                    self.scopes.push(Scope::default());
                    let then = self.stmt_inner(body);
                    self.scopes.truncate(depth);
                    self.control = control;
                    self.reach = self.flow.or(self.reach, skipped);
                    let then = then?;
                    stmts.push(hir::Stmt::If {
                        condition,
                        then,
                        otherwise: Vec::new(),
                    });
                }
                Ok(stmts)
            }
            StmtKind::Expr(value) => {
                if let ExprKind::Call { callee, args } = &value.kind
                    && let Some(Value::Control {
                        target,
                        restart,
                        owner,
                    }) = self.symbol(callee)?
                {
                    if !args.is_empty() {
                        return Err(Self::error(
                            "E212",
                            "scope operations take no arguments",
                            value.span,
                        ));
                    }
                    if owner != self.owner {
                        return Err(Self::error(
                            "E201",
                            "scope operation cannot cross a function",
                            value.span,
                        ));
                    }
                    let index = self
                        .frames
                        .iter()
                        .position(|frame| frame.id == target)
                        .ok_or_else(|| {
                            Self::error(
                                "E201",
                                "scope operation escaped its target lifetime",
                                value.span,
                            )
                        })?;
                    let entered = self.reach;
                    let first = self.frames[index].first;
                    if restart {
                        let start = self.frames[index].start;
                        for frame in &self.frames[..index] {
                            for (name, slots) in &frame.slots {
                                for slot in slots {
                                    if slot.order >= start
                                        && self.flow.overlap(self.reach, slot.guard)
                                    {
                                        let unsupported = || {
                                            Diagnostic::unsupported(
                                                "restart after an emission into an enclosing scope",
                                                value.span,
                                            )
                                        };
                                        let expected = frame
                                            .expected
                                            .as_ref()
                                            .filter(|_| !frame.partial)
                                            .ok_or_else(unsupported)?;
                                        crate::borrow_contract::type_weight(
                                            expected,
                                            &mut self.flow,
                                            value.span,
                                        )?;
                                        let (_, ty) = crate::borrow::slot(expected, name)
                                            .ok_or_else(unsupported)?;
                                        if !crate::borrow::carried::eligible(
                                            ty,
                                            &mut self.flow,
                                            value.span,
                                        )? || *ty != slot.ty
                                        {
                                            return Err(unsupported());
                                        }
                                        let size = name.as_ref().map_or(0, String::len) + 1;
                                        if !self.flow.spend(size.saturating_mul(
                                            self.proofs.carried.len().checked_ilog2().unwrap_or(0)
                                                as usize
                                                + 2,
                                        )) {
                                            return Err(Diagnostic::unsupported(
                                                "carried publication budget exhausted",
                                                value.span,
                                            ));
                                        }
                                        let key = (frame.id, name.clone());
                                        if self.proofs.carried.len()
                                            >= crate::borrow::carried::MAX_SLOTS
                                            && !self.proofs.carried.contains_key(&key)
                                        {
                                            return Err(Diagnostic::unsupported(
                                                "carried publication budget exhausted",
                                                value.span,
                                            ));
                                        }
                                        self.proofs.carried.entry(key).or_insert(
                                            crate::borrow::carried::Slot {
                                                ty: ty.clone(),
                                                span: value.span,
                                            },
                                        );
                                    }
                                }
                            }
                        }
                        self.forget_mutable();
                    } else {
                        let leaves = self.frames[index].leaves;
                        self.frames[index].leaves = self.flow.or(leaves, self.reach);
                    }
                    self.reach = FALSE;
                    return Ok(vec![if restart {
                        if self.restarts >= 65_536
                            || !self.flow.spend(
                                self.proofs.frontiers.len().checked_ilog2().unwrap_or(0) as usize
                                    + 4,
                            )
                        {
                            return Err(Diagnostic::unsupported(
                                "restart site budget exhausted",
                                value.span,
                            ));
                        }
                        let site = self.restarts;
                        self.restarts += 1;
                        self.proofs.frontiers.insert(
                            site,
                            crate::borrow::frontier::Frontier {
                                target,
                                first,
                                entered,
                            },
                        );
                        hir::Stmt::Restart { target, site }
                    } else {
                        hir::Stmt::Leave(target)
                    }]);
                }
                Ok(vec![hir::Stmt::Expr(self.expr(value, None)?)])
            }
            StmtKind::Forward { .. } => Err(Self::error(
                "E221",
                "forward signature is not in a definition group",
                stmt.span,
            )),
        }
    }

    pub(crate) fn emit(
        &mut self,
        label: Option<&str>,
        name: Option<&str>,
        annotation: Option<&ast::TypeExpr>,
        mutable: bool,
        value: &ast::Expr,
        span: Span,
    ) -> Result<Vec<hir::Stmt>> {
        if label.is_none()
            && name.is_some()
            && self.owner == 0
            && self.scopes.len() == self.module.depth
            && self
                .frames
                .last()
                .is_some_and(|frame| frame.id == self.module.block)
            && self.pending_query(value)?.is_some()
        {
            if annotation
                .map(|ty| self.spec(ty))
                .transpose()?
                .is_none_or(|ty| matches!(ty, super::Spec::Descriptor(_)))
            {
                return Err(Diagnostic::unsupported(
                    "proof descriptor metadata exports",
                    span,
                ));
            }
            return Err(Self::error(
                "E223",
                "proof descriptors cannot enter runtime exports",
                span,
            ));
        }
        if self.export_type_value(label, name, annotation, mutable, value, span)? {
            return Ok(Vec::new());
        }
        if self.export_function(label, name, annotation, mutable, value, span)? {
            return Ok(Vec::new());
        }
        if mutable && name.is_none() {
            return Err(Diagnostic::unsupported("mutable primary emissions", span));
        }
        let target = match label {
            Some(name) => self.label(name, span)?,
            None => self.frames.last().expect("frame").id,
        };
        let frame = self
            .frames
            .iter()
            .find(|frame| frame.id == target)
            .expect("frame");
        let union = frame
            .expected
            .as_ref()
            .filter(|ty| Self::record_union(ty))
            .cloned();
        let record = if name.is_none() && matches!(frame.expected, Some(Type::Record { .. })) {
            frame.expected.clone()
        } else {
            None
        };
        let expected = match (&frame.expected, name) {
            (Some(ty), name) if Self::record_union(ty) => Self::union_slot(ty, name, mutable),
            (Some(Type::Record { primary, .. }), None) => Some(*primary.clone()),
            (Some(Type::Record { fields, .. }), Some(name)) => Some(
                fields
                    .iter()
                    .find(|field| field.name == name)
                    .map(|field| field.ty.clone())
                    .ok_or_else(|| {
                        Self::error(
                            "E207",
                            format!("field `{name}` is not in the expected record"),
                            span,
                        )
                    })?,
            ),
            (Some(ty), None) => Some(ty.clone()),
            (Some(_), Some(name)) => {
                return Err(Self::error(
                    "E207",
                    format!("scalar result cannot contain field `{name}`"),
                    span,
                ));
            }
            (None, _) => None,
        };
        let annotated = annotation.map(|ty| self.construct_type(ty)).transpose()?;
        if let (Some(expected), Some(annotated)) = (&expected, &annotated)
            && !expected.accepts(annotated)
        {
            return Err(Self::error(
                "E207",
                "emission annotation differs from the required slot type",
                span,
            ));
        }
        let value = if let Some(record) = record {
            self.composed(value, record, annotated.as_ref().or(expected.as_ref()))?
        } else if union.is_some() {
            if let Some(annotated) = annotated.as_ref() {
                self.expr(value, Some(annotated))?
            } else {
                self.expression(value, expected.as_ref())?
            }
        } else {
            self.expr(value, annotated.as_ref().or(expected.as_ref()))?
        };
        if value.ty == Type::Never {
            return Ok(vec![hir::Stmt::Expr(value)]);
        }
        if mutable && value.ty.has_reference() && !value.ty.fixed_borrowed_value() {
            return Err(Diagnostic::unsupported(
                "mutable reference-bearing record fields",
                span,
            ));
        }
        let mut stmts = Vec::new();
        if name.is_none()
            && matches!(value.ty, Type::Record { .. })
            && !union.as_ref().is_some_and(|ty| ty.accepts(&value.ty))
        {
            let ty = value.ty.clone();
            let id = self.local(ty.clone());
            self.track_record_references(id, &value, false)?;
            let local = hir::Expr {
                kind: hir::ExprKind::Local(id),
                ty: ty.clone(),
                span,
            };
            stmts.push(hir::Stmt::Bind { id, value });
            let Type::Record { primary, fields } = ty else {
                unreachable!()
            };
            self.write_slot(target, None, *primary.clone(), mutable, None, span)?;
            stmts.push(self.emission(
                target,
                None,
                hir::Expr {
                    kind: hir::ExprKind::Primary(Box::new(local.clone())),
                    ty: *primary,
                    span,
                },
            ));
            for (index, field) in fields.into_iter().enumerate() {
                self.write_slot(
                    target,
                    Some(field.name.clone()),
                    field.ty.clone(),
                    field.mutable,
                    None,
                    span,
                )?;
                stmts.push(self.emission(
                    target,
                    Some(field.name),
                    hir::Expr {
                        kind: hir::ExprKind::Field {
                            value: Box::new(local.clone()),
                            index,
                        },
                        ty: field.ty,
                        span,
                    },
                ));
            }
            self.composed_inputs(target, &stmts)?;
            if self.record_pointees.contains_key(&id) || self.record_cells.contains_key(&id) {
                self.record_compositions.entry(target).or_default().push(id);
            }
        } else if let Some(name) = name {
            let ty = value.ty.clone();
            let id = self.local(ty.clone());
            self.track_reference(id, &value, false)?;
            self.track_reference_cell(id, &value, false)?;
            self.track_record_references(id, &value, false)?;
            if self.control || self.derived_expr(&value) {
                self.mark_derived(id);
            }
            if !mutable && let Some(fact) = self.list_fact(&value) {
                self.lengths.insert(id, fact);
            }
            self.write_slot(
                target,
                Some(name.into()),
                ty.clone(),
                mutable,
                self.list_fact(&value),
                span,
            )?;
            self.declare(
                name,
                Value::Local {
                    id,
                    ty: ty.clone(),
                    mutable,
                    owner: self.owner,
                    constant: if mutable || self.derived_local(id) {
                        None
                    } else {
                        self.constant(&value)
                    },
                },
                span,
            )?;
            if !mutable {
                self.export_input(target, name, id, &value);
            }
            stmts.push(hir::Stmt::Bind { id, value });
            let emission = self.emission(
                target,
                Some(name.into()),
                hir::Expr {
                    kind: hir::ExprKind::Local(id),
                    ty,
                    span,
                },
            );
            let hir::Stmt::Emit { id: emitted, .. } = &emission else {
                unreachable!()
            };
            let alias = self.slot_alias(id, target, name, *emitted, mutable, span)?;
            stmts.push(emission);
            stmts.push(alias);
        } else {
            self.write_slot(
                target,
                None,
                value.ty.clone(),
                mutable,
                self.list_fact(&value),
                span,
            )?;
            let emission = self.emission(target, None, value);
            self.primary_input(&emission);
            stmts.push(emission);
        }
        Ok(stmts)
    }

    pub(crate) fn emission(
        &mut self,
        target: hir::BlockId,
        field: Option<String>,
        value: hir::Expr,
    ) -> hir::Stmt {
        let id = self.proofs.emissions.len();
        self.proofs.emissions.insert(id, self.reach);
        hir::Stmt::Emit {
            id,
            target,
            field,
            value,
        }
    }

    pub(crate) fn write_slot(
        &mut self,
        target: usize,
        name: Option<String>,
        ty: Type,
        mutable: bool,
        list: Option<crate::list::Fact>,
        span: Span,
    ) -> Result<()> {
        if self.reach == FALSE {
            return Ok(());
        }
        if target == self.module.block
            && !self.module.values.is_empty()
            && let Some(name) = &name
        {
            if !self.flow.spend(name.len() + self.module.values.len() + 1) {
                return Err(Diagnostic::unsupported(
                    "module export budget exhausted",
                    span,
                ));
            }
            if self.module.values.contains_key(name) {
                return Err(Self::error(
                    "E205",
                    format!("module export `{name}` is already emitted"),
                    span,
                ));
            }
        }
        let frame = self
            .frames
            .iter()
            .find(|frame| frame.id == target)
            .expect("frame");
        let expected = match (&frame.expected, &name) {
            (Some(Type::Record { primary, .. }), None) => Some(primary.as_ref()),
            (Some(Type::Record { fields, .. }), Some(name)) => fields
                .iter()
                .find(|field| &field.name == name)
                .map(|field| &field.ty),
            (Some(ty), None) => Some(ty),
            _ => None,
        };
        if let (Some(Type::Record { fields, .. }), Some(name)) = (&frame.expected, &name)
            && fields
                .iter()
                .find(|field| &field.name == name)
                .is_some_and(|field| field.mutable != mutable)
        {
            return Err(Self::error(
                "E206",
                "result field mutability differs from its declaration",
                span,
            ));
        }
        if let Some(expected) = expected
            && !expected.accepts(&ty)
            && !frame.expected.as_ref().is_some_and(Self::record_union)
        {
            return Err(Self::error(
                "E207",
                format!("result slot has type {ty:?}, expected {expected:?}"),
                span,
            ));
        }
        if frame.expected.is_some()
            && expected.is_none()
            && !frame.expected.as_ref().is_some_and(Self::record_union)
        {
            return Err(Self::error(
                "E207",
                "result field is absent from the expected type",
                span,
            ));
        }
        let slots = &mut self
            .frames
            .iter_mut()
            .find(|frame| frame.id == target)
            .expect("frame")
            .slots;
        let writes = slots.entry(name.clone()).or_default();
        if writes
            .iter()
            .any(|slot| self.flow.overlap(slot.guard, self.reach))
        {
            return Err(Self::error(
                "E205",
                format!(
                    "result {} may be emitted twice",
                    name.as_ref()
                        .map(|name| format!("field `{name}`"))
                        .unwrap_or_else(|| "primary".into())
                ),
                span,
            ));
        }
        writes.push(Slot {
            ty,
            mutable,
            guard: self.reach,
            order: self.writes,
            list,
        });
        self.writes += 1;
        Ok(())
    }
}
