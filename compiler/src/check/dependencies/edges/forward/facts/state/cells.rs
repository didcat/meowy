use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) enum Data {
    Unavailable,
    Uninitialized,
    Partial,
    Conditional {
        domain: Domain,
        origin: Option<Origin>,
    },
    Initialized {
        version: Option<Version>,
        domain: Option<Domain>,
        origin: Option<Origin>,
    },
}

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct Cell {
    pub(in super::super) data: Data,
    pub(in super::super) control: bool,
}

impl Cell {
    pub(super) fn same(&self, other: &Self, flow: &mut Flow, span: Span) -> Result<bool> {
        if !flow.spend(2) {
            return Err(budget(span));
        }
        if self.control != other.control
            || std::mem::discriminant(&self.data) != std::mem::discriminant(&other.data)
        {
            return Ok(false);
        }
        if matches!(
            self.data,
            Data::Initialized { .. } | Data::Conditional { .. }
        ) && !flow.spend(self.weight() + other.weight())
        {
            return Err(budget(span));
        }
        Ok(self == other)
    }

    pub(super) fn weight(&self) -> usize {
        1 + match &self.data {
            Data::Conditional { domain, origin } => domain.weight() + usize::from(origin.is_some()),
            Data::Initialized {
                version,
                domain,
                origin,
            } => {
                usize::from(version.is_some())
                    + domain.as_ref().map_or(0, Domain::weight)
                    + usize::from(origin.is_some())
            }
            _ => 0,
        }
    }

    pub(super) fn bottom(&self) -> bool {
        matches!(&self.data, Data::Initialized { domain: Some(domain), .. } if *domain == Domain::BOTTOM)
    }

    pub(super) fn validate(
        &self,
        owner: usize,
        local: hir::LocalId,
        flow: &mut Flow,
        span: Span,
    ) -> Result<()> {
        if self.weight() > MAX_EDGES
            || !flow.spend(self.weight().saturating_mul(2).saturating_add(1))
        {
            return Err(budget(span));
        }
        let (version, domain, origin) = match &self.data {
            Data::Initialized {
                version,
                domain,
                origin,
            } => (*version, domain.as_ref(), *origin),
            Data::Conditional { domain, origin } => {
                if *domain == Domain::BOTTOM {
                    return Err(invalid(span));
                }
                (None, Some(domain), *origin)
            }
            _ => return Ok(()),
        };
        if version.is_some_and(|version| version.owner != owner || version.local != local)
            || domain.is_some_and(|domain| !domain.valid())
            || (domain.is_none() && origin.is_some())
            || origin.is_some_and(|origin| match origin {
                Origin::Parameter(function, _) => function.checked_add(1) != Some(owner),
                Origin::Expression(source, _) => source != owner,
                Origin::Version(version) => version.owner != owner,
            })
        {
            return Err(invalid(span));
        }
        Ok(())
    }
}

impl Data {
    pub(super) fn conditional(
        left: &Self,
        right: &Self,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Self> {
        let (domain, origin) = match (left.value(), right.value()) {
            (Some((domain, origin)), None) if *right == Self::Uninitialized => {
                (domain.clone(), origin)
            }
            (None, Some((domain, origin))) if *left == Self::Uninitialized => {
                (domain.clone(), origin)
            }
            (Some((a, x)), Some((b, y))) => (a.join(b, flow, span)?, if x == y { x } else { None }),
            _ => return Ok(Self::Partial),
        };
        Ok(Self::Conditional { domain, origin })
    }

    pub(self) fn value(&self) -> Option<(&Domain, Option<Origin>)> {
        match self {
            Self::Initialized {
                domain: Some(domain),
                origin,
                ..
            }
            | Self::Conditional { domain, origin } => Some((domain, *origin)),
            _ => None,
        }
    }
}
