use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Observed {
    pub(crate) input: PointId,
    pub(crate) composed: Option<Composition>,
    pub(crate) targets: Vec<Target>,
    pub(crate) control: bool,
    pub(crate) initialized: Vec<bool>,
    pub(crate) result: bool,
}

impl Checker {
    pub(in super::super) fn record_emission_effect(
        &mut self,
        owner: usize,
        stage: Stage,
        effects: &mut Effects,
        limit: usize,
        parts: &mut usize,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof emission-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof emission-effect identity mismatch", span);
        let (id, part) = stage;
        if !self.flow.spend(
            self.emissions.len().checked_ilog2().unwrap_or(0) as usize
                + effects.len().checked_ilog2().unwrap_or(0) as usize * 2
                + 8,
        ) {
            return Err(budget());
        }
        let op = self.emissions.get(&id).ok_or_else(invalid)?;
        let bytes = name_bytes(&op.targets, &mut self.flow, span)?;
        let len = op.targets.len();
        if !self.flow.spend(bytes * 2 + len * 3 + 6) {
            return Err(budget());
        }
        if op.owner != owner || len == 0 || part.is_some_and(|part| part >= len) {
            return Err(invalid());
        }
        let fresh = if let Some((prior_owner, effect)) = effects.get(&id) {
            let Effect::Emission(prior) = effect else {
                return Err(invalid());
            };
            if *prior_owner != owner
                || prior.input != op.input
                || prior.composed != op.composed
                || prior.targets != op.targets
                || prior.control != op.control
                || prior.initialized.len() != len
            {
                return Err(invalid());
            }
            false
        } else {
            if effects.len() >= limit || len * 2 + bytes > *parts {
                return Err(budget());
            }
            true
        };
        let (_, Effect::Emission(observed)) = effects.entry(id).or_insert_with(|| {
            (
                owner,
                Effect::Emission(Observed {
                    input: op.input,
                    composed: op.composed,
                    targets: op.targets.clone(),
                    control: op.control,
                    initialized: vec![false; len],
                    result: false,
                }),
            )
        }) else {
            unreachable!()
        };
        match part {
            Some(part) => observed.initialized[part] = true,
            None => observed.result = true,
        }
        if fresh {
            *parts -= len * 2 + bytes;
        }
        Ok(())
    }
}
