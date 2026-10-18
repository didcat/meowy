use super::super::effects::Effect;
use super::*;
use domain::Domain;

#[derive(Debug)]
pub(super) struct Seed<'a> {
    pub(super) source: &'a (usize, Effect),
    pub(super) declared: Option<Domain>,
    pub(super) value: Option<Domain>,
}

pub(super) type Literals<'a> = BTreeMap<PointId, Seed<'a>>;

impl Checker {
    pub(super) fn scalar_literal_seeds<'a>(
        &mut self,
        reports: &'a Reports,
        span: Span,
        parts: usize,
    ) -> Result<(Literals<'a>, usize)> {
        let budget = || Diagnostic::unsupported("proof scalar-seed budget exhausted", span);
        if reports.effects.len() > MAX_EDGES || !self.flow.spend(reports.effects.len() + 1) {
            return Err(budget());
        }
        let mut seeds = Literals::new();
        let mut parts = parts.min(MAX_EDGES);
        for (&point, source) in &reports.effects {
            if !matches!(source.1, Effect::Scalar(_)) {
                continue;
            }
            let mut seed = Seed {
                source,
                declared: None,
                value: None,
            };
            let (owner, Effect::Scalar(observed)) = seed.source else {
                unreachable!()
            };
            self.validate_scalar_report(reports, point, *owner, observed, span)?;
            seed.declared = Domain::scalar(observed.ty, &mut self.flow, span)?;
            if observed.result {
                seed.value = observed
                    .value
                    .map(|value| Domain::literal(value, &mut self.flow, span))
                    .transpose()?;
            }
            let weight = 1
                + seed.declared.as_ref().map_or(0, Domain::weight)
                + seed.value.as_ref().map_or(0, Domain::weight);
            parts = parts.checked_sub(weight).ok_or_else(budget)?;
            if !self
                .flow
                .spend(seeds.len().checked_ilog2().unwrap_or(0) as usize + weight)
            {
                return Err(budget());
            }
            seeds.insert(point, seed);
        }
        Ok((seeds, parts))
    }
}

#[cfg(test)]
mod tests;
