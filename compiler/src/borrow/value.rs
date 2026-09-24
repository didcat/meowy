use super::{Checker, Expr, ExprKind, FALSE, Flow, Projection, Result, State, Step, Type, Value};

impl Checker<'_> {
    pub(crate) fn expression(&mut self, expr: &Expr) -> Result<Value> {
        let mut flow = Flow::new();
        let state = match &expr.kind {
            ExprKind::TemporaryBorrow {
                id,
                statement,
                value,
            } => {
                let result = self.expression(value)?;
                flow = result.flow;
                if !flow.next {
                    State::absent()
                } else {
                    if self.program.locals.get(*id) != Some(&value.ty)
                        || self.proofs.temporaries.get(id) != Some(statement)
                        || !self.statements.contains_key(statement)
                    {
                        return Err(Self::unsupported(expr.span));
                    }
                    result.state.borrowed(
                        super::Source::Temporary {
                            id: *id,
                            statement: *statement,
                            fields: Vec::new(),
                        },
                        &value.ty,
                        self.guards,
                        expr.span,
                    )?
                }
            }
            ExprKind::Borrow(place) => {
                if !self.locals.contains_key(&place.root) {
                    return Err(Self::unsupported(expr.span));
                }
                let mut ty = self
                    .program
                    .locals
                    .get(place.root)
                    .ok_or_else(|| Self::unsupported(expr.span))?;
                for index in &place.fields {
                    let Type::Record { fields, .. } = ty else {
                        return Err(Self::unsupported(expr.span));
                    };
                    ty = &fields
                        .get(*index)
                        .ok_or_else(|| Self::unsupported(expr.span))?
                        .ty;
                }
                if expr.ty.pointee() != Some(ty) {
                    return Err(Self::unsupported(expr.span));
                }
                let path = place
                    .fields
                    .iter()
                    .map(|index| Step::Slot(index + 1))
                    .collect::<Vec<_>>();
                let state = self.locals[&place.root].state.select(&path, self.guards);
                state.borrowed(self.proofs.source(place), ty, self.guards, expr.span)?
            }
            ExprKind::ExclusivePath { place, path } => {
                let element = self
                    .proofs
                    .exclusive_path_type(self.program, place, path, self.guards, expr.span)
                    .cloned()
                    .ok_or_else(|| Self::unsupported(expr.span))?;
                let diverges = path.iter().any(|step| {
                    matches!(step, crate::hir::WriteStep::Index(step) if step.index.ty == Type::Never)
                });
                if expr.ty != Type::Exclusive(Box::new(element.clone()))
                    && !(expr.ty == Type::Never && diverges)
                {
                    return Err(Self::unsupported(expr.span));
                }
                let mut source = self.proofs.source(place);
                self.live(&source, expr.span)?;
                for step in path {
                    if !self.guards.spend(place.fields.len() + path.len() + 1) {
                        return Err(State::budget(expr.span));
                    }
                    if !flow.next {
                        break;
                    }
                    match step {
                        crate::hir::WriteStep::Field(index) => {
                            source = source.project(&[Projection::Field(*index)]);
                        }
                        crate::hir::WriteStep::Index(step) => {
                            flow.append(self.expression(&step.index)?.flow);
                            source = source.project(&[Projection::Element]);
                        }
                    }
                }
                if flow.next {
                    self.live(&source, expr.span)?;
                    State::default().borrowed(source, &element, self.guards, expr.span)?
                } else {
                    State::absent()
                }
            }
            ExprKind::Reborrow { site, value, .. }
            | ExprKind::ElementBorrow { site, value, .. } => {
                let result = self.expression(value)?;
                flow = result.flow;
                let fields = match &expr.kind {
                    ExprKind::Reborrow { fields, .. } => {
                        if !self.guards.spend(fields.len() + 1) {
                            return Err(State::budget(expr.span));
                        }
                        fields
                            .iter()
                            .copied()
                            .map(Projection::Field)
                            .collect::<Vec<_>>()
                    }
                    ExprKind::ElementBorrow { index, .. } => {
                        if flow.next {
                            flow.append(self.expression(index)?.flow);
                        }
                        vec![Projection::Element]
                    }
                    _ => unreachable!(),
                };
                let (Type::Reference(ty) | Type::Exclusive(ty)) = &value.ty else {
                    return Err(Self::unsupported(expr.span));
                };
                let target = crate::borrow_contract::projected_type(ty, &fields)
                    .ok_or_else(|| Self::unsupported(expr.span))?;
                if expr.ty != Type::Never && expr.ty.pointee() != Some(target) {
                    return Err(Self::unsupported(expr.span));
                }
                let state = result
                    .state
                    .reborrowed(&fields, target, self.guards, expr.span)?;
                self.reserve_origins(state.weight() + 1, expr.span)?;
                self.facts.reborrows.insert(*site, state.clone());
                state
            }
            ExprKind::Local(id) => {
                if self.proofs.variable(*id) && !self.proofs.versioned(self.program, *id) {
                    State::unknown(&expr.ty, self.guards, expr.span)?
                } else {
                    let state = self
                        .locals
                        .get(id)
                        .map(|storage| storage.state.clone())
                        .ok_or_else(|| Self::unsupported(expr.span))?;
                    self.live_value(&state, expr.span)?;
                    state
                }
            }
            ExprKind::Coerce { value } => {
                let result = self.expression(value)?;
                flow = result.flow;
                result
                    .state
                    .convert(&value.ty, &expr.ty, self.guards, expr.span)?
            }
            ExprKind::Deref(_) | ExprKind::Field { .. } | ExprKind::Primary(_) => {
                let result = self.read(expr, &[])?;
                flow = result.flow;
                result.state
            }
            ExprKind::Unary { value, .. }
            | ExprKind::StringSize(value)
            | ExprKind::ListSize(value) => {
                flow = self.expression(value)?.flow;
                State::unknown(&expr.ty, self.guards, expr.span)?
            }
            ExprKind::TypeTest { value, .. } => {
                flow = self.inspect(value)?;
                if flow.next {
                    self.inspect_tags(expr, value)?;
                }
                State::unknown(&expr.ty, self.guards, expr.span)?
            }
            ExprKind::List { values, .. } => {
                for value in values {
                    if !flow.next {
                        break;
                    }
                    let next = self.expression(value)?;
                    if next.flow.next {
                        self.unbounded(
                            &next.state,
                            "allocator bounds in list elements",
                            value.span,
                        )?;
                    }
                    flow.append(next.flow);
                }
                State::unknown(&expr.ty, self.guards, expr.span)?
            }
            ExprKind::ListIndex { value, index } | ExprKind::ListAdd { value, item: index } => {
                let left = self.expression(value)?;
                flow = left.flow;
                if flow.next {
                    self.unbounded(
                        &left.state,
                        "allocator bounds in list operations",
                        value.span,
                    )?;
                    let right = self.expression(index)?;
                    if right.flow.next {
                        self.unbounded(
                            &right.state,
                            "allocator bounds in list operations",
                            index.span,
                        )?;
                    }
                    flow.append(right.flow);
                }
                State::unknown(&expr.ty, self.guards, expr.span)?
            }
            ExprKind::Binary {
                op, left, right, ..
            } => {
                flow = self.expression(left)?.flow;
                if flow.next && self.merging && ["&&", "||"].contains(&op.as_str()) {
                    let guard = self.condition(left)?;
                    let guard = if op == "&&" {
                        guard
                    } else {
                        self.guards.not(guard)
                    };
                    let next = self.conditional(
                        guard,
                        expr.span,
                        |checker| Ok(checker.expression(right)?.flow),
                        |_| Ok(Flow::new()),
                    )?;
                    flow.append(next);
                } else if flow.next {
                    let skip = match (&left.kind, op.as_str()) {
                        (ExprKind::Bool(value), "&&") => Some(!value),
                        (ExprKind::Bool(value), "||") => Some(*value),
                        _ => None,
                    };
                    if skip != Some(true) {
                        let mut next = self.expression(right)?.flow;
                        if skip.is_none() && ["&&", "||"].contains(&op.as_str()) {
                            next.next = true;
                        }
                        flow.append(next);
                    }
                }
                State::unknown(&expr.ty, self.guards, expr.span)?
            }
            ExprKind::Call { site, args, .. } => {
                let mut inputs = Vec::new();
                for arg in args {
                    if !flow.next {
                        break;
                    }
                    let value = self.expression(arg)?;
                    inputs.push((&arg.ty, value.state));
                    flow.append(value.flow);
                }
                let entered = self
                    .proofs
                    .calls
                    .get(site)
                    .copied()
                    .ok_or_else(|| Self::unsupported(expr.span))?;
                let state = if flow.next {
                    for (_, state) in &inputs {
                        self.live_argument(state, expr.span, entered)?;
                    }
                    if crate::borrow_contract::reference_call(
                        &expr.ty,
                        inputs.iter().map(|(ty, _)| *ty),
                    ) {
                        let (state, transfers) = crate::borrow_contract::returns::call(
                            &expr.ty,
                            &inputs,
                            self.guards,
                            expr.span,
                        )?;
                        self.reserve_origins(transfers.len() * 2 + 1, expr.span)?;
                        self.facts.returns.insert(*site, transfers);
                        state
                    } else {
                        crate::borrow_contract::call(&expr.ty, &inputs, self.guards, expr.span)?
                    }
                } else {
                    State::absent()
                };
                let normal = self.guards.and(state.present, state.proof);
                let post = self.guards.or(self.guards.not(entered), normal);
                self.assumed = self.guards.and(self.assumed, post);
                self.reserve_origins(state.weight() + 1, expr.span)?;
                self.facts.calls.insert(*site, state.clone());
                state
            }
            ExprKind::Print { parts, .. } | ExprKind::Panic { parts } => {
                for part in parts {
                    if !flow.next {
                        break;
                    }
                    flow.append(self.expression(part)?.flow);
                }
                if matches!(expr.kind, ExprKind::Panic { .. }) {
                    flow.next = false;
                }
                State::default()
            }
            ExprKind::Block(block) => {
                let value = self.block(block)?;
                flow = value.flow;
                value.state
            }
            ExprKind::Null
            | ExprKind::Bool(_)
            | ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Heap => State::default(),
        };
        let assumptions = self.assumptions();
        let proof = self.guards.and(state.proof, assumptions);
        if self.guards.and(state.present, proof) == FALSE {
            flow.next = false;
        }
        if flow.next {
            self.complete(&expr.ty, &state, expr.span)?;
        }
        if expr.ty == Type::Never {
            flow.next = false;
        }
        if self.merging && flow.next {
            let normal = self.guards.and(state.present, state.proof);
            self.assumed = self.guards.and(self.assumed, normal);
        }
        Ok(Value { state, flow })
    }
}
