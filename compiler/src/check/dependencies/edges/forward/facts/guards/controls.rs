use super::*;
use crate::{check::dependencies::Point, flow::Flow};

pub(super) fn dependency(
    mut id: PointId,
    branch: Branch,
    points: &[Point],
    branches: &HashMap<PointId, Branch>,
    flow: &mut Flow,
    span: Span,
) -> Result<Option<bool>> {
    let mut control = Some(branch.control);
    loop {
        if !flow.spend(3) {
            return Err(budget(span));
        }
        let point = points.get(id).ok_or_else(|| invalid(span))?;
        if !point.complete || point.owner != branch.owner {
            return Err(invalid(span));
        }
        let arm = matches!(point.kind, PointKind::Then | PointKind::Else);
        let Some(parent) = point.parent else {
            if arm {
                return Err(invalid(span));
            }
            return Ok(control);
        };
        if parent >= id {
            return Err(invalid(span));
        }
        if arm {
            if !flow.spend(2) {
                return Err(budget(span));
            }
            let outer = points.get(parent).ok_or_else(|| invalid(span))?;
            if !matches!(
                outer.kind,
                PointKind::Match | PointKind::And | PointKind::Or
            ) {
                return Err(invalid(span));
            }
            let inherited = branches.get(&parent);
            if inherited.is_some_and(|source| source.owner != branch.owner) {
                return Err(invalid(span));
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

#[cfg(test)]
mod tests;
