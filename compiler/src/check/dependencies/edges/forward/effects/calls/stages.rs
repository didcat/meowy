use super::*;

pub(super) fn matches(call: &Invocation, effect: &Effect) -> bool {
    matches!(effect, Effect::Call {
        site, function, args, may_return, control, operation, result,
    } if *site == call.site
        && *function == call.function
        && *args == call.args
        && *may_return == call.may_return
        && *control == call.control
        && (*operation || *result)
        && (!result || *may_return))
}

impl Checker {
    pub(in super::super::super) fn validate_call_report(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        effect: &Effect,
        span: Span,
    ) -> Result<()> {
        let invalid = || Diagnostic::unsupported("proof call-effect identity mismatch", span);
        let Effect::Call { site, .. } = effect else {
            return Err(invalid());
        };
        let call = self.checked_call_effect(reports, *site, span)?;
        if call.point != id || call.owner != owner || !matches(call, effect) {
            return Err(invalid());
        }
        Ok(())
    }

    pub(in super::super) fn record_call_effect(
        &mut self,
        stage: Stage,
        effects: &mut Effects,
        limit: usize,
        parts: &mut usize,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof call-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof call-effect identity mismatch", span);
        if !self.flow.spend(
            self.invocations.len().checked_ilog2().unwrap_or(0) as usize
                + effects.len().checked_ilog2().unwrap_or(0) as usize * 2
                + 10,
        ) {
            return Err(budget());
        }
        let call = self.invocations.get(&stage.site).ok_or_else(invalid)?;
        if call.args.len() > crate::check::dependencies::sequences::MAX_ITEMS
            || !self.flow.spend(call.args.len() + 1)
        {
            return Err(budget());
        }
        if call.point != stage.point
            || call.owner != stage.owner
            || call.site != stage.site
            || call.function != stage.function
            || call.args.len() != stage.count
            || call.may_return != stage.may_return
            || call.control != stage.control
            || (stage.kind == Kind::Result && !stage.may_return)
        {
            return Err(invalid());
        }
        let fresh = if let Some((owner, prior)) = effects.get(&stage.point) {
            if *owner != stage.owner || !matches(call, prior) {
                return Err(invalid());
            }
            false
        } else {
            if effects.len() >= limit || call.args.len() > *parts {
                return Err(budget());
            }
            true
        };
        let (
            _,
            Effect::Call {
                operation, result, ..
            },
        ) = effects.entry(stage.point).or_insert_with(|| {
            (
                stage.owner,
                Effect::Call {
                    site: call.site,
                    function: call.function,
                    args: call.args.clone(),
                    may_return: call.may_return,
                    control: call.control,
                    operation: false,
                    result: false,
                },
            )
        })
        else {
            unreachable!()
        };
        if fresh {
            *parts -= call.args.len();
        }
        match stage.kind {
            Kind::Operation => *operation = true,
            Kind::Result => *result = true,
        }
        Ok(())
    }
}
