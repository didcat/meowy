use super::restart::{Headers, same, weight};
use super::{
    BTreeMap, Block, Checker, Facts, Guards, LocalId, Program, Proofs, Result, Span, State,
    Storage, TRUE,
};

pub(crate) const MAX_PASSES: usize = 64;

pub(crate) fn body(
    program: &Program,
    guards: &mut Guards,
    proofs: &Proofs,
    block: &Block,
    params: &[LocalId],
    used: usize,
) -> Result<(Facts, usize)> {
    let plan = super::mutable::check(block, program, guards, params, proofs)?;
    let span = Span::default();
    let mut headers = Headers::new();
    let mut choices = super::activity::Choices::new();
    for _ in 0..MAX_PASSES {
        let size = weight(&headers)
            + super::activity::weight(&choices)
            + super::published::fixed_weight(&plan.fixed)
            + super::changing::weight(&plan.changing)
            + super::changing::weight(&plan.late)
            + super::changing::weight(&plan.refresh);
        let seeds = size.saturating_mul(2);
        if used.saturating_add(seeds) > super::MAX_FACT_ORIGINS
            || !guards.spend(size + plan.restarts.len() + 1)
        {
            return Err(State::budget(span));
        }
        let mut checker = Checker {
            choices: choices.clone(),
            headers: headers.clone(),
            restarts: plan.restarts.clone(),
            targets: BTreeMap::new(),
            merging: plan.merging,
            program,
            guards,
            proofs,
            locals: BTreeMap::new(),
            blocks: Vec::new(),
            types: BTreeMap::new(),
            results: BTreeMap::new(),
            published: BTreeMap::new(),
            writes: BTreeMap::new(),
            scopes: Vec::new(),
            facts: Facts {
                fixed_published: plan.fixed.clone(),
                changing_published: plan.changing.clone(),
                late_published: plan.late.clone(),
                refresh_published: plan.refresh.clone(),
                ..Facts::default()
            },
            origins: used,
            assumed: TRUE,
            assumed_scopes: Vec::new(),
            inputs: params.iter().copied().collect(),
            statements: BTreeMap::new(),
        };
        checker.reserve_origins(seeds, span)?;
        if checker.merging {
            checker.facts.merging.insert(block.id);
        }
        for id in params {
            let ty = &program.locals[*id];
            let mut state = crate::borrow_contract::input(*id, ty, checker.guards, span)?;
            checker.link_tags(*id, ty, &mut state, span)?;
            checker.complete(ty, &state, span)?;
            checker.reserve_origins(state.weight() + 1, span)?;
            checker.assumed = checker.guards.and(checker.assumed, state.proof);
            checker.facts.locals.insert(*id, state.clone());
            checker.locals.insert(
                *id,
                Storage {
                    block: block.id,
                    state,
                },
            );
        }
        checker.block(block)?;
        if same(&headers, &checker.headers, checker.guards, span)? {
            checker.facts.headers = checker.headers;
            return Ok((checker.facts, checker.origins));
        }
        headers = checker.headers;
        choices = checker.choices;
    }
    Err(super::Diagnostic::unsupported(
        "restart origin fixed-point budget exhausted",
        span,
    ))
}

pub(crate) fn check(program: &Program, guards: &mut Guards, proofs: &Proofs) -> Result<Facts> {
    let (mut facts, mut used) = body(program, guards, proofs, &program.body, &[], 0)?;
    for function in &program.functions {
        let (next, count) = body(
            program,
            guards,
            proofs,
            &function.body,
            &function.params,
            used,
        )?;
        used = count;
        let entries = next.inspections.len()
            + next.locals.len()
            + next.blocks.len()
            + next.calls.len()
            + next.call_inputs.len()
            + next.returns.len()
            + next.reborrows.len()
            + next.headers.len()
            + next.header_inputs.len()
            + next.restart_inputs.len()
            + next.published_inputs.len()
            + next.published_restarts.len()
            + next.fixed_published.len()
            + next.changing_published.len()
            + next.late_published.len()
            + next.refresh_published.len()
            + next.merging.len();
        let lookup = program.locals.len().checked_ilog2().unwrap_or(0) as usize + 1;
        if !guards.spend(entries.saturating_mul(lookup)) {
            return Err(State::budget(Span::default()));
        }
        facts.inspections.extend(next.inspections);
        facts.locals.extend(next.locals);
        facts.blocks.extend(next.blocks);
        facts.calls.extend(next.calls);
        facts.call_inputs.extend(next.call_inputs);
        facts.returns.extend(next.returns);
        facts.reborrows.extend(next.reborrows);
        facts.headers.extend(next.headers);
        facts.header_inputs.extend(next.header_inputs);
        facts.restart_inputs.extend(next.restart_inputs);
        facts.published_inputs.extend(next.published_inputs);
        facts.published_restarts.extend(next.published_restarts);
        facts.fixed_published.extend(next.fixed_published);
        facts.changing_published.extend(next.changing_published);
        facts.late_published.extend(next.late_published);
        facts.refresh_published.extend(next.refresh_published);
        facts.merging.extend(next.merging);
    }
    Ok(facts)
}
