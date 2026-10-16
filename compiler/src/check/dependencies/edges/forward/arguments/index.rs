use super::*;

impl Checker {
    pub(super) fn call_argument_index(
        &mut self,
        reports: &Reports,
        span: Span,
        limit: usize,
    ) -> Result<Arguments> {
        let budget = || Diagnostic::unsupported("proof call-argument index budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof call-argument identity mismatch", span);
        let calls = self.call_effect_index(span)?;
        let mut args = Arguments::new();
        for site in calls.into_values() {
            if !self
                .flow
                .spend(self.invocations.len().checked_ilog2().unwrap_or(0) as usize + 2)
            {
                return Err(budget());
            }
            let count = self.invocations.get(&site).ok_or_else(invalid)?.args.len();
            if count > crate::check::dependencies::sequences::MAX_ITEMS {
                return Err(budget());
            }
            let size = args
                .len()
                .checked_add(count)
                .filter(|size| *size <= limit.min(MAX_EDGES))
                .ok_or_else(budget)?;
            if !self
                .flow
                .spend(count * (size.checked_ilog2().unwrap_or(0) as usize * 2 + 3) + 1)
            {
                return Err(budget());
            }
            let inputs = self.checked_call_inputs(reports, site, span)?;
            let owner = inputs.call.owner;
            for (position, (param, point)) in inputs.pairs.enumerate() {
                if args
                    .insert(
                        point,
                        Argument {
                            site,
                            position,
                            param,
                            owner,
                            entry: false,
                            normal: false,
                        },
                    )
                    .is_some()
                {
                    return Err(invalid());
                }
            }
        }
        Ok(args)
    }
}
