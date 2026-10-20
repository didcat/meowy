use super::*;
use crate::check::dependencies::{Branch, Point};
use crate::flow::Flow;

pub(super) enum Error {
    Budget,
    Identity,
}

impl Error {
    pub(super) fn diagnostic(self, context: &str, span: Span) -> Diagnostic {
        let reason = match self {
            Self::Budget => "budget exhausted",
            Self::Identity => "identity mismatch",
        };
        Diagnostic::unsupported(format!("proof {context} {reason}"), span)
    }
}

pub(super) fn dependency(
    mut id: PointId,
    owner: usize,
    dependent: bool,
    points: &[Point],
    branches: &HashMap<PointId, Branch>,
    flow: &mut Flow,
) -> std::result::Result<Option<bool>, Error> {
    let mut control = Some(dependent);
    loop {
        if !flow.spend(3) {
            return Err(Error::Budget);
        }
        let point = points.get(id).ok_or(Error::Identity)?;
        if !point.complete || point.owner != owner {
            return Err(Error::Identity);
        }
        let arm = matches!(point.kind, PointKind::Then | PointKind::Else);
        let Some(parent) = point.parent else {
            if arm {
                return Err(Error::Identity);
            }
            return Ok(control);
        };
        if parent >= id {
            return Err(Error::Identity);
        }
        if arm {
            if !flow.spend(2) {
                return Err(Error::Budget);
            }
            let outer = points.get(parent).ok_or(Error::Identity)?;
            if !matches!(
                outer.kind,
                PointKind::Match | PointKind::And | PointKind::Or
            ) {
                return Err(Error::Identity);
            }
            let inherited = branches.get(&parent);
            if inherited.is_some_and(|source| source.owner != owner) {
                return Err(Error::Identity);
            }
            control = match (control, inherited.map(|source| source.control)) {
                (Some(true), _) | (_, Some(true)) => Some(true),
                (Some(false), Some(false)) => Some(false),
                _ => None,
            };
        }
        id = parent;
    }
}
