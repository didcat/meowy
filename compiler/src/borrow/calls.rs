use super::{CallId, Checker, Expr, Flow, Result, State, Value};

impl Checker<'_> {
    pub(crate) fn call(&mut self, expr: &Expr, site: CallId, args: &[Expr]) -> Result<Value> {
        let mut flow = Flow::new();
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
            .get(&site)
            .copied()
            .ok_or_else(|| Self::unsupported(expr.span))?;
        let state = if flow.next {
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
                self.reserve_origins(transfers.len() * 2 + 1, expr.span)?;
                self.facts.returns.insert(site, transfers);
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
        self.facts.calls.insert(site, state.clone());
        Ok(Value { state, flow })
    }
}
