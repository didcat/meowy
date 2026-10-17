use crate::ast::Span;
use crate::borrow::{BlockId, Guards, MAX_FACT_ORIGINS, Result, State, Type};
use crate::flow::Guard;
use crate::hir::{CallId, FunctionId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Header {
    pub(crate) site: CallId,
    pub(crate) function: FunctionId,
    pub(crate) body: BlockId,
    pub(crate) count: usize,
    pub(crate) entered: Guard,
}

pub(crate) struct Inputs {
    pub(crate) header: Header,
    pub(crate) normal: usize,
    pub(crate) args: Vec<(usize, State)>,
}

impl Inputs {
    pub(crate) fn capture(
        header: Header,
        normal: usize,
        args: Vec<(&Type, State)>,
        flow: &mut Guards,
        limit: usize,
        span: Span,
    ) -> Result<(Self, usize)> {
        let limit = limit.min(MAX_FACT_ORIGINS);
        if header.count > MAX_FACT_ORIGINS
            || args.len().saturating_add(1) > limit
            || !flow.spend(args.len() + 1)
        {
            return Err(State::budget(span));
        }
        let mut cost = args.len() + 1;
        for (_, state) in &args {
            if !flow.spend(state.size().saturating_add(1)) {
                return Err(State::budget(span));
            }
            cost = cost.saturating_add(state.weight());
            if cost > limit {
                return Err(State::budget(span));
            }
        }
        if !flow.spend(cost) {
            return Err(State::budget(span));
        }
        let result = Self {
            header,
            normal,
            args: args
                .into_iter()
                .enumerate()
                .map(|(position, (_, state))| (position, state))
                .collect(),
        };
        if result.args.len() > result.header.count
            || result.normal > result.args.len()
            || (result.normal == result.args.len() && result.normal != result.header.count)
            || (result.normal < result.args.len() && result.normal + 1 != result.args.len())
        {
            return Err(crate::diagnostic::Diagnostic::unsupported(
                "borrow call-input prefix mismatch",
                span,
            ));
        }
        Ok((result, cost))
    }
}

#[cfg(test)]
mod tests;
