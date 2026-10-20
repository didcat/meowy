use super::*;
use crate::{check::dependencies::Point, flow::Flow};

pub(super) fn dependency(
    id: PointId,
    branch: Branch,
    points: &[Point],
    branches: &HashMap<PointId, Branch>,
    flow: &mut Flow,
    span: Span,
) -> Result<Option<bool>> {
    super::super::super::controls::dependency(
        id,
        branch.owner,
        branch.control,
        points,
        branches,
        flow,
    )
    .map_err(|error| error.diagnostic("scalar-guard", span))
}

#[cfg(test)]
mod tests;
