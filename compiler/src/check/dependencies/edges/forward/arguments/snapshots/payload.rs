use crate::ast::Span;
use crate::borrow::calls::Inputs;
use crate::borrow_value::MAX_PARTS;
use crate::diagnostic::Diagnostic;
use crate::flow::Flow;

pub(super) fn budget(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof borrow-call budget exhausted", span)
}

pub(super) fn invalid(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof borrow-call identity mismatch", span)
}

pub(super) fn validate(
    snapshot: &Inputs,
    flow: &mut Flow,
    span: Span,
    parts: usize,
) -> Result<usize, Diagnostic> {
    let count = snapshot.args.len();
    if count.saturating_add(1) > parts || !flow.spend(count + 1) {
        return Err(budget(span));
    }
    if count > snapshot.header.count
        || snapshot.normal > count
        || (snapshot.normal == count && count != snapshot.header.count)
        || (snapshot.normal < count && snapshot.normal + 1 != count)
        || snapshot.header.entered >> 1 >= flow.nodes.len()
    {
        return Err(invalid(span));
    }
    let mut cost = count + 1;
    for (position, (stored, state)) in snapshot.args.iter().enumerate() {
        if state.size() > MAX_PARTS || !flow.spend(state.size() + 1) {
            return Err(budget(span));
        }
        cost = cost.saturating_add(state.weight());
        if cost > parts || !flow.spend(state.weight() + 1) {
            return Err(budget(span));
        }
        if position != *stored
            || state.present >> 1 >= flow.nodes.len()
            || state.proof >> 1 >= flow.nodes.len()
            || state
                .origins
                .iter()
                .chain(&state.bounds)
                .any(|origin| origin.guard >> 1 >= flow.nodes.len())
            || state
                .active
                .iter()
                .any(|active| active.guard >> 1 >= flow.nodes.len())
        {
            return Err(invalid(span));
        }
    }
    Ok(cost)
}
