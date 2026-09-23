use crate::ast::Span;
use crate::borrow_value::{MAX_PARTS, Result, State};
use crate::flow::{FALSE, Flow, Guard, TRUE};
use crate::hir::{ReferenceMode, Type};

#[derive(Clone, Debug)]
pub(crate) struct Transfer {
    pub(crate) input: usize,
    pub(crate) guard: Guard,
}

pub(crate) fn candidate(result: &Type, input: &Type) -> bool {
    input.pointee() == result.pointee()
        && (result.reference_mode() == Some(ReferenceMode::Shared)
            || input.reference_mode() == Some(ReferenceMode::Exclusive))
}

pub(crate) fn call(
    ty: &Type,
    args: &[(&Type, State)],
    flow: &mut Flow,
    span: Span,
) -> Result<(State, Vec<Transfer>)> {
    if !super::reference_call(ty, args.iter().map(|(ty, _)| *ty)) || !flow.spend(args.len() + 1) {
        return Err(State::budget(span));
    }
    let mut result = State::unknown(ty, flow, span)?;
    let mut candidates = Vec::new();
    for (input, (from, state)) in args.iter().enumerate() {
        if candidate(ty, from) {
            if candidates.len() == MAX_PARTS {
                return Err(State::budget(span));
            }
            candidates.push((input, (from, state)));
        }
    }
    let mut rest = TRUE;
    let mut covered = FALSE;
    let mut transfers = Vec::new();
    for (index, (input, (_, state))) in candidates.iter().enumerate() {
        let chosen = if index + 1 == candidates.len() {
            rest
        } else {
            let fresh = flow.fresh();
            let chosen = flow.and(rest, fresh);
            rest = flow.and(rest, flow.not(fresh));
            chosen
        };
        let mut supplied = FALSE;
        for origin in &state.origins {
            if !origin.component.is_empty() || !flow.spend(origin.weight() + 1) {
                return Err(State::budget(span));
            }
            let guard = flow.and(chosen, state.present);
            let guard = flow.and(guard, origin.guard);
            let guard = flow.and(guard, state.proof);
            if guard != FALSE {
                let mut origin = origin.clone();
                origin.guard = guard;
                result.origins.push(origin);
                supplied = flow.or(supplied, guard);
            }
            if result.size() > MAX_PARTS {
                return Err(State::budget(span));
            }
        }
        covered = flow.or(covered, supplied);
        transfers.push(Transfer {
            input: *input,
            guard: supplied,
        });
    }
    result.proof = flow.and(result.proof, covered);
    for (_, state) in args {
        result.proof = flow.and(result.proof, state.proof);
        for origin in state.origins.iter().chain(&state.bounds) {
            if !flow.spend(origin.weight() + 1) {
                return Err(State::budget(span));
            }
            let guard = flow.and(origin.guard, state.present);
            if guard != FALSE {
                let mut bound = origin.clone();
                bound.guard = guard;
                result.bounds.push(bound);
            }
            if result.size() > MAX_PARTS {
                return Err(State::budget(span));
            }
        }
    }
    if flow.exceeded() || !flow.spend(result.weight() + transfers.len() * 2) {
        return Err(State::budget(span));
    }
    Ok((result, transfers))
}
