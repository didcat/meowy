use super::*;
use literals::Literals;
use sources::{Inputs, Producer};
use std::collections::BTreeSet;
use versions::{Definition, Origin, Value, Versions};

impl Checker {
    pub(super) fn scalar_copies<'a>(
        &mut self,
        mut versions: Versions<'a>,
        inputs: &Inputs,
        literals: &Literals,
        span: Span,
        parts: usize,
    ) -> Result<(Versions<'a>, usize)> {
        if versions.len() > MAX_EDGES
            || inputs.len() > versions.len()
            || !self.flow.spend(inputs.len() + 1)
        {
            return Err(budget(span));
        }
        let mut parts = parts.min(MAX_EDGES);
        let mut done = BTreeSet::new();
        for &root in inputs.keys() {
            let mut pending = Vec::new();
            let mut seen = BTreeSet::new();
            let mut local = root;
            loop {
                if !self.flow.spend(
                    done.len().checked_ilog2().unwrap_or(0) as usize
                        + seen.len().checked_ilog2().unwrap_or(0) as usize
                        + inputs.len().checked_ilog2().unwrap_or(0) as usize
                        + 4,
                ) {
                    return Err(budget(span));
                }
                if done.contains(&local) {
                    break;
                }
                if !seen.insert(local) {
                    return Err(invalid(span));
                }
                if pending.len() >= versions.len() {
                    return Err(budget(span));
                }
                pending.push(local);
                let Some(sources::Input {
                    producer: Producer::Read(_, from),
                    ..
                }) = inputs.get(&local)
                else {
                    break;
                };
                local = from.local;
            }
            while let Some(local) = pending.pop() {
                if !self.flow.spend(
                    inputs.len().checked_ilog2().unwrap_or(0) as usize
                        + versions.len().checked_ilog2().unwrap_or(0) as usize * 3
                        + literals.len().checked_ilog2().unwrap_or(0) as usize
                        + done.len().checked_ilog2().unwrap_or(0) as usize
                        + 8,
                ) {
                    return Err(budget(span));
                }
                let target = versions.get(&local).ok_or_else(|| invalid(span))?;
                if let Some(input) = inputs.get(&local) {
                    if !matches!(target.version.definition, Definition::Bind(_))
                        || target.value.is_some()
                    {
                        return Err(invalid(span));
                    }
                    let source = match input.producer {
                        Producer::Literal(point) => {
                            let seed = literals.get(&point).ok_or_else(|| invalid(span))?;
                            if seed.source.0 != target.version.owner {
                                return Err(invalid(span));
                            }
                            seed.value.as_ref().map(|domain| {
                                (domain, Origin::Expression(seed.source.0, point), false)
                            })
                        }
                        Producer::Read(_, version) => {
                            let source =
                                versions.get(&version.local).ok_or_else(|| invalid(span))?;
                            if source.version != version || version.owner != target.version.owner {
                                return Err(invalid(span));
                            }
                            source
                                .value
                                .as_ref()
                                .map(|value| (&value.domain, value.origin, value.control))
                        }
                    };
                    if let Some((domain, origin, control)) = source {
                        let weight = 1 + domain.weight();
                        parts = parts.checked_sub(weight).ok_or_else(|| budget(span))?;
                        if !self.flow.spend(weight) {
                            return Err(budget(span));
                        }
                        let value = Value {
                            domain: domain.clone(),
                            origin,
                            control: control || input.control,
                        };
                        versions.get_mut(&local).ok_or_else(|| invalid(span))?.value = Some(value);
                    }
                }
                done.insert(local);
            }
        }
        Ok((versions, parts))
    }
}

pub(super) fn budget(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-copy budget exhausted", span)
}

pub(super) fn invalid(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-copy identity mismatch", span)
}

#[cfg(test)]
mod tests;
