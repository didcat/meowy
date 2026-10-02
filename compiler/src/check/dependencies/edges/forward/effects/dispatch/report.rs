use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Observed {
    pub(crate) input: PointId,
    pub(crate) local: crate::hir::LocalId,
    pub(crate) block: crate::hir::BlockId,
    pub(crate) normal: bool,
    pub(crate) control: bool,
    pub(crate) initialized: bool,
    pub(crate) result: bool,
}

impl Checker {
    pub(in super::super) fn record_dispatch_effect(
        &mut self,
        owner: usize,
        port: Port,
        effects: &mut Effects,
        limit: usize,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof dispatch-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof dispatch-effect identity mismatch", span);
        let id = match port {
            Port::Operation(id) | Port::Normal(id) => id,
            _ => return Err(invalid()),
        };
        if !self.flow.spend(
            self.dispatch_ops.len().checked_ilog2().unwrap_or(0) as usize
                + effects.len().checked_ilog2().unwrap_or(0) as usize * 2
                + 10,
        ) {
            return Err(budget());
        }
        let op = self.dispatch_ops.get(&id).ok_or_else(invalid)?;
        if op.owner != owner || !op.input_normal || (!op.normal && port == Port::Normal(id)) {
            return Err(invalid());
        }
        if let Some((prior_owner, effect)) = effects.get(&id) {
            let Effect::Dispatch(prior) = effect else {
                return Err(invalid());
            };
            if *prior_owner != owner
                || prior.input != op.input
                || prior.local != op.local
                || prior.block != op.block
                || prior.normal != op.normal
                || prior.control != op.control
            {
                return Err(invalid());
            }
        } else if effects.len() >= limit {
            return Err(budget());
        }
        let (_, Effect::Dispatch(observed)) = effects.entry(id).or_insert_with(|| {
            (
                owner,
                Effect::Dispatch(Observed {
                    input: op.input,
                    local: op.local,
                    block: op.block,
                    normal: op.normal,
                    control: op.control,
                    initialized: false,
                    result: false,
                }),
            )
        }) else {
            unreachable!()
        };
        match port {
            Port::Operation(_) => observed.initialized = true,
            Port::Normal(_) => observed.result = true,
            _ => unreachable!(),
        }
        Ok(())
    }
}
