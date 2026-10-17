use super::{CallId, Checker, Expr, ExprKind, Flow, MAX_FACT_ORIGINS, Result, State, Value};

mod inputs;

pub(crate) use inputs::{Header, Inputs};

impl Checker<'_> {
    pub(crate) fn call(&mut self, expr: &Expr, site: CallId, args: &[Expr]) -> Result<Value> {
        if args.len() > MAX_FACT_ORIGINS || !self.guards.spend(args.len() + 1) {
            return Err(State::budget(expr.span));
        }
        let ExprKind::Call { id, .. } = expr.kind else {
            return Err(Self::unsupported(expr.span));
        };
        let body = *self
            .blocks
            .first()
            .ok_or_else(|| Self::unsupported(expr.span))?;
        let mut completed = 0;
        let mut flow = Flow::new();
        let mut inputs = Vec::new();
        for arg in args {
            if !flow.next {
                break;
            }
            let value = self.expression(arg)?;
            completed += usize::from(value.flow.next);
            inputs.push((&arg.ty, value.state));
            flow.append(value.flow);
        }
        let entered = self
            .proofs
            .calls
            .get(&site)
            .copied()
            .ok_or_else(|| Self::unsupported(expr.span))?;
        let (state, transfers) = if flow.next {
            for (_, state) in &inputs {
                self.live_argument(state, expr.span, entered)?;
            }
            if crate::borrow_contract::reference_call(&expr.ty, inputs.iter().map(|(ty, _)| *ty)) {
                let (state, transfers) = crate::borrow_contract::returns::call(
                    &expr.ty,
                    &inputs,
                    self.guards,
                    expr.span,
                )?;
                (state, Some(transfers))
            } else {
                (
                    crate::borrow_contract::call(&expr.ty, &inputs, self.guards, expr.span)?,
                    None,
                )
            }
        } else {
            (State::absent(), None)
        };
        let normal = self.guards.and(state.present, state.proof);
        let post = self.guards.or(self.guards.not(entered), normal);
        self.assumed = self.guards.and(self.assumed, post);
        let output = state.weight() + 1 + transfers.as_ref().map_or(0, |items| items.len() * 2 + 1);
        let parts = MAX_FACT_ORIGINS
            .checked_sub(self.origins.saturating_add(output))
            .ok_or_else(|| State::budget(expr.span))?;
        let header = Header {
            function: id,
            body,
            count: args.len(),
            entered,
        };
        if !self
            .guards
            .spend(self.facts.call_inputs.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(State::budget(expr.span));
        }
        if self
            .facts
            .call_inputs
            .get(&site)
            .is_some_and(|prior| prior.header != header)
        {
            return Err(Self::unsupported(expr.span));
        }
        let (snapshot, cost) =
            Inputs::capture(header, completed, inputs, self.guards, parts, expr.span)?;
        self.reserve_origins(output + cost, expr.span)?;
        if let Some(transfers) = transfers {
            self.facts.returns.insert(site, transfers);
        }
        self.facts.calls.insert(site, state.clone());
        self.facts.call_inputs.insert(site, snapshot);
        Ok(Value { state, flow })
    }
}

#[cfg(test)]
mod tests;
