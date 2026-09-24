use super::{
    BTreeSet, Block, Checker, Exit, ExprKind, FALSE, Flow, Result, Span, State, Stmt, TRUE, Type,
    Value, slot,
};
use crate::hir::WriteStep;

impl Checker<'_> {
    pub(crate) fn block(&mut self, block: &Block) -> Result<Value> {
        self.blocks.push(block.id);
        self.types.insert(block.id, block.ty.clone());
        self.results.insert(block.id, State::absent());
        self.scopes.push(Vec::new());
        self.assumed_scopes.push(self.assumed);
        if self.merging {
            self.enter_target(block.id, Span::default())?;
            if self.restarts.contains(&block.id) {
                self.enter_restart(block.id, Span::default())?;
            }
        }
        let mut flow = self.statements(&block.stmts)?;
        let span = Span { start: 0, end: 0 };
        if self.merging {
            self.complete_target(block.id, flow.next, span)?;
        }
        let complete = self
            .proofs
            .completions
            .get(&block.id)
            .copied()
            .ok_or_else(|| Self::unsupported(span))?;
        let mut state = self.results.remove(&block.id).expect("result state");
        let slots: Vec<_> = match &block.ty {
            Type::Record { primary, fields } => std::iter::once((None, primary.as_ref()))
                .chain(
                    fields
                        .iter()
                        .map(|field| (Some(field.name.clone()), &field.ty)),
                )
                .collect(),
            ty => vec![(None, ty)],
        };
        for (field, ty) in slots {
            let written = self
                .writes
                .remove(&(block.id, field.clone()))
                .unwrap_or(FALSE);
            let missing = self.guards.and(complete, self.guards.not(written));
            if missing != FALSE && ty.accepts(&Type::Null) {
                let prefix = slot(&block.ty, &field).expect("result slot").0;
                let default = State::default()
                    .convert(&Type::Null, ty, self.guards, span)?
                    .under(missing, self.guards)
                    .prefix(&prefix);
                state.merge(default, self.guards, span)?;
            }
        }
        state.present = complete;
        let assumptions = self.assumptions();
        state.proof = self.guards.and(state.proof, assumptions);
        state = self.finish_published(block.id, state, span)?;
        self.complete(&block.ty, &state, span)?;
        if state.size() != 0 || state.proof != TRUE {
            self.reserve_origins(state.weight() + 1, span)?;
            self.facts.blocks.insert(block.id, state.clone());
        }
        self.close_scope();
        self.blocks.pop();
        self.end_published(block.id, span)?;
        self.types.remove(&block.id);
        let effective = self.guards.and(state.present, state.proof);
        flow.next = effective != FALSE;
        flow.exits.remove(&Exit::Leave(block.id));
        flow.exits.remove(&Exit::Restart(block.id));
        Ok(Value { state, flow })
    }

    pub(crate) fn branch(&mut self, stmts: &[Stmt]) -> Result<Flow> {
        self.scopes.push(Vec::new());
        self.assumed_scopes.push(self.assumed);
        let flow = self.statements(stmts)?;
        self.close_scope();
        Ok(flow)
    }

    pub(crate) fn statements(&mut self, stmts: &[Stmt]) -> Result<Flow> {
        let mut flow = Flow::new();
        for stmt in stmts {
            if !flow.next {
                break;
            }
            let next = match stmt {
                Stmt::Statement { id, stmts } => {
                    let owner = *self.blocks.last().expect("statement owner");
                    if !self.guards.spend(1) || self.statements.insert(*id, owner).is_some() {
                        return Err(State::budget(Span::default()));
                    }
                    let result = self.statements(stmts);
                    self.statements.remove(id);
                    result?
                }
                Stmt::Bind { id, value } => {
                    let result = self.expression(value)?;
                    if result.flow.next {
                        self.bind(*id, result.state, value.span)?;
                    }
                    result.flow
                }
                Stmt::SlotAlias {
                    id,
                    target,
                    field,
                    mutable,
                } => {
                    let alias = self
                        .proofs
                        .aliases
                        .get(id)
                        .ok_or_else(|| Self::unsupported(Span::default()))?;
                    if !self.locals.contains_key(id)
                        || !self.types.contains_key(target)
                        || alias.target != *target
                        || alias.field != *field
                        || alias.mutable != *mutable
                    {
                        return Err(Self::unsupported(Span::default()));
                    }
                    Flow::new()
                }
                Stmt::Assign { id, value } => {
                    let result = self.expression(value)?;
                    if result.flow.next {
                        if self.proofs.versioned(self.program, *id) {
                            if !self.proofs.mutable.contains(id)
                                || value.ty != self.program.locals[*id]
                            {
                                return Err(Self::unsupported(value.span));
                            }
                            self.sync_alias(*id, &[], &result.state, value.span)?;
                            self.reserve_origins(result.state.weight() + 1, value.span)?;
                            self.locals
                                .get_mut(id)
                                .ok_or_else(|| Self::unsupported(value.span))?
                                .state = result.state;
                        } else {
                            self.unbounded(&result.state, "allocator assignment", value.span)?;
                            if !result.state.origins.is_empty()
                                || self.program.locals[*id].has_reference()
                            {
                                return Err(Self::unsupported(value.span));
                            }
                        }
                    }
                    result.flow
                }
                Stmt::Store {
                    target,
                    value,
                    span,
                } => {
                    let pointer = self.expression(target)?;
                    let mut result = pointer.flow;
                    if result.next {
                        let next = self.expression(value)?;
                        if next.flow.next {
                            self.live_value(&pointer.state, *span)?;
                        }
                        result.append(next.flow);
                    }
                    result
                }
                Stmt::SetPath {
                    id,
                    path,
                    value,
                    span,
                } => {
                    if !self.locals.contains_key(id) {
                        return Err(Self::unsupported(*span));
                    }
                    let mut ty = &self.program.locals[*id];
                    let mut mutable = self.proofs.mutable.contains(id);
                    let tracked = self.proofs.versioned(self.program, *id);
                    if (ty.has_reference() && !tracked) || path.is_empty() {
                        return Err(Self::unsupported(*span));
                    }
                    let mut prefix = Vec::new();
                    let mut result = Flow::new();
                    for step in path {
                        if !result.next {
                            break;
                        }
                        if !self.guards.spend(1) {
                            return Err(State::budget(*span));
                        }
                        match step {
                            WriteStep::Field(index) => {
                                let Type::Record { fields, .. } = ty else {
                                    return Err(Self::unsupported(*span));
                                };
                                let field =
                                    fields.get(*index).ok_or_else(|| Self::unsupported(*span))?;
                                mutable = field.mutable;
                                ty = &field.ty;
                                prefix.push(super::Step::Slot(index + 1));
                            }
                            WriteStep::Index(step) => {
                                if tracked {
                                    return Err(Self::unsupported(*span));
                                }
                                let Type::List { element, .. } = ty else {
                                    return Err(Self::unsupported(step.span));
                                };
                                ty = element;
                                result.append(self.expression(&step.index)?.flow);
                            }
                        }
                    }
                    if result.next {
                        if !mutable {
                            return Err(Self::unsupported(*span));
                        }
                        let next = self.expression(value)?;
                        if next.flow.next {
                            if (!tracked && !next.state.origins.is_empty()) || value.ty != *ty {
                                return Err(Self::unsupported(*span));
                            }
                            if tracked {
                                let current = self
                                    .locals
                                    .get(id)
                                    .ok_or_else(|| Self::unsupported(*span))?
                                    .state
                                    .clone();
                                let state =
                                    current.replaced(&prefix, next.state, self.guards, *span)?;
                                self.complete(&self.program.locals[*id], &state, *span)?;
                                self.sync_alias(*id, &prefix, &state, *span)?;
                                self.reserve_origins(state.weight() + 1, *span)?;
                                self.locals
                                    .get_mut(id)
                                    .ok_or_else(|| Self::unsupported(*span))?
                                    .state = state;
                            } else {
                                self.unbounded(
                                    &next.state,
                                    "allocator bounds in field or indexed assignment",
                                    *span,
                                )?;
                            }
                        }
                        result.append(next.flow);
                    }
                    result
                }
                Stmt::Emit {
                    id,
                    target,
                    field,
                    value,
                } => {
                    let result = self.expression(value)?;
                    if result.flow.next {
                        self.emit(*id, *target, field, result.state, &value.ty, value.span)?;
                    }
                    result.flow
                }
                Stmt::If {
                    condition,
                    then,
                    otherwise,
                    ..
                } => {
                    let mut result = self.expression(condition)?.flow;
                    if result.next {
                        let next = if self.merging {
                            let guard = self.condition(condition)?;
                            self.conditional(
                                guard,
                                condition.span,
                                |checker| checker.statements(then),
                                |checker| checker.statements(otherwise),
                            )?
                        } else {
                            match &condition.kind {
                                ExprKind::Bool(true) => self.branch(then)?,
                                ExprKind::Bool(false) => self.branch(otherwise)?,
                                _ => {
                                    let mut next = self.branch(then)?;
                                    next.merge(self.branch(otherwise)?);
                                    next
                                }
                            }
                        };
                        result.append(next);
                    }
                    result
                }
                Stmt::Leave(id) | Stmt::Restart { target: id, .. } => {
                    if self.merging && matches!(stmt, Stmt::Leave(_)) {
                        self.leave_target(*id, Span::default())?;
                    } else if self.merging {
                        let Stmt::Restart { site, .. } = stmt else {
                            unreachable!()
                        };
                        self.restart_target(*id, *site, Span::default())?;
                    }
                    Flow {
                        next: false,
                        exits: BTreeSet::from([if matches!(stmt, Stmt::Leave(_)) {
                            Exit::Leave(*id)
                        } else {
                            Exit::Restart(*id)
                        }]),
                    }
                }
                Stmt::Expr(value) => self.expression(value)?.flow,
            };
            flow.append(next);
        }
        Ok(flow)
    }
}
