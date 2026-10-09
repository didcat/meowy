use super::*;
use crate::check::dependencies::{
    BinaryClass as Class, BinaryPlan, BinaryTypes, ScalarKind, SequenceSource,
    binaries::types::MAX_COUNT,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Projection(usize),
    Operation,
    Result,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stage {
    pub(super) point: PointId,
    pub(super) owner: usize,
    pub(super) inputs: [PointId; 2],
    pub(super) op: &'static str,
    pub(super) types: BinaryTypes,
    pub(super) plan: BinaryPlan,
    pub(super) control: bool,
    pub(super) kind: Kind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Observed {
    pub(crate) inputs: [PointId; 2],
    pub(crate) op: &'static str,
    pub(crate) types: BinaryTypes,
    pub(crate) plan: BinaryPlan,
    pub(crate) control: bool,
    pub(crate) projected: [bool; 2],
    pub(crate) operation: bool,
    pub(crate) result: bool,
}

pub(super) fn signature(op: &str, types: BinaryTypes, plan: BinaryPlan) -> bool {
    for ty in types.inputs.into_iter().chain([types.result]) {
        match ty {
            Class::Scalar(ScalarKind::Int { bits, .. })
            | Class::SharedScalar(ScalarKind::Int { bits, .. })
                if !matches!(bits, 8 | 16 | 32 | 64) =>
            {
                return false;
            }
            Class::Scalar(ScalarKind::Float { bits })
            | Class::SharedScalar(ScalarKind::Float { bits })
                if !matches!(bits, 32 | 64) =>
            {
                return false;
            }
            Class::Record { fields } if fields > MAX_COUNT => return false,
            Class::List { capacity } if capacity > crate::list::MAX_CAPACITY => return false,
            Class::Union { members } if !(2..=MAX_COUNT).contains(&members) => return false,
            _ => (),
        }
    }
    let normal = types.inputs.map(|ty| ty != Class::Never);
    if plan.normal != normal
        || plan.equality != (normal.into_iter().all(|value| value) && matches!(op, "==" | "!="))
    {
        return false;
    }
    if !normal.into_iter().all(|value| value) {
        return types.result == Class::Never && !plan.checked;
    }
    let left = types.inputs[0];
    if left != types.inputs[1] {
        return false;
    }
    let integer = matches!(left, Class::Scalar(ScalarKind::Int { .. }));
    let numeric = integer || matches!(left, Class::Scalar(ScalarKind::Float { .. }));
    let compare = matches!(op, "==" | "!=" | "<" | "<=" | ">" | ">=");
    let valid = match op {
        "+" | "-" | "*" | "/" => numeric,
        "%" | "&" | "|" | "^" => integer,
        "<" | "<=" | ">" | ">=" => numeric || left == Class::Scalar(ScalarKind::String),
        "==" | "!=" => true,
        _ => false,
    };
    valid
        && types.result
            == if compare {
                Class::Scalar(ScalarKind::Bool)
            } else {
                left
            }
        && plan.checked == (integer && matches!(op, "+" | "-" | "*" | "/" | "%"))
}

impl Checker {
    pub(super) fn record_binary_effect(
        &mut self,
        stage: Stage,
        effects: &mut Effects,
        limit: usize,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof binary-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof binary-effect identity mismatch", span);
        if !self
            .flow
            .spend(effects.len().checked_ilog2().unwrap_or(0) as usize * 2 + 12)
        {
            return Err(budget());
        }
        if matches!(stage.kind, Kind::Projection(step) if step >= 2) {
            return Err(invalid());
        }
        if let Some((owner, effect)) = effects.get(&stage.point) {
            let Effect::Binary(prior) = effect else {
                return Err(invalid());
            };
            if *owner != stage.owner
                || prior.inputs != stage.inputs
                || prior.op != stage.op
                || prior.types != stage.types
                || prior.plan != stage.plan
                || prior.control != stage.control
            {
                return Err(invalid());
            }
        } else if effects.len() >= limit {
            return Err(budget());
        }
        let (_, Effect::Binary(binary)) = effects.entry(stage.point).or_insert_with(|| {
            (
                stage.owner,
                Effect::Binary(Observed {
                    inputs: stage.inputs,
                    op: stage.op,
                    types: stage.types,
                    plan: stage.plan,
                    control: stage.control,
                    projected: [false; 2],
                    operation: false,
                    result: false,
                }),
            )
        }) else {
            unreachable!()
        };
        match stage.kind {
            Kind::Projection(step) => binary.projected[step] = true,
            Kind::Operation => binary.operation = true,
            Kind::Result => binary.result = true,
        }
        Ok(())
    }

    pub(super) fn binary_effect_stage(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<Option<Stage>> {
        let budget = || Diagnostic::unsupported("proof binary-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof binary-effect identity mismatch", span);
        let id = match port {
            Port::Projection { point, .. } | Port::Operation(point) | Port::Normal(point) => point,
            _ => return Ok(None),
        };
        if !self
            .flow
            .spend(self.binaries.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some(op) = self.binaries.get(&id) else {
            return Ok(None);
        };
        if !self
            .flow
            .spend(self.sequences.len().checked_ilog2().unwrap_or(0) as usize + 28)
        {
            return Err(budget());
        }
        let symbol = match op.op.as_str() {
            "+" => "+",
            "-" => "-",
            "*" => "*",
            "/" => "/",
            "%" => "%",
            "&" => "&",
            "|" => "|",
            "^" => "^",
            "==" => "==",
            "!=" => "!=",
            "<" => "<",
            "<=" => "<=",
            ">" => ">",
            ">=" => ">=",
            _ => return Err(invalid()),
        };
        let point = self.points.get(id).ok_or_else(invalid)?;
        let sequence = self
            .sequences
            .get(&SequenceSource::Expr(id))
            .ok_or_else(invalid)?;
        if !signature(symbol, op.types, op.plan)
            || op.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != op.span
            || sequence.owner != owner
            || sequence.items.as_slice() != op.inputs.map(Some)
            || op.inputs[0] == op.inputs[1]
        {
            return Err(invalid());
        }
        for input in op.inputs {
            if !self.points.get(input).is_some_and(|input| {
                input.complete
                    && input.owner == owner
                    && input.parent == Some(id)
                    && input.block == point.block
                    && matches!(input.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            }) {
                return Err(invalid());
            }
        }
        let mut edges = [Edge::new(Port::Entry(id), Port::Entry(op.inputs[0]), Route::Next); 5];
        let mut count = 1;
        let mut left = Port::Normal(op.inputs[0]);
        if op.plan.primary[0] {
            let stage = Port::Projection { point: id, step: 0 };
            edges[count] = Edge::new(left, stage, Route::Next);
            count += 1;
            left = stage;
        }
        let link = [Edge::new(left, Port::Entry(op.inputs[1]), Route::Next)];
        let ready = op.plan.normal.into_iter().all(|value| value);
        if op.plan.normal[0] {
            let mut right = Port::Normal(op.inputs[1]);
            if op.plan.primary[1] {
                let stage = Port::Projection { point: id, step: 1 };
                edges[count] = Edge::new(right, stage, Route::Next);
                count += 1;
                right = stage;
            }
            if ready {
                edges[count] = Edge::new(right, Port::Operation(id), Route::Next);
                edges[count + 1] = Edge::new(
                    Port::Operation(id),
                    Port::Normal(id),
                    if op.plan.checked {
                        Route::Checked
                    } else {
                        Route::Next
                    },
                );
                count += 2;
            }
        }
        if op.edges.as_slice() != &edges[..count]
            || sequence.edges.as_slice() != &link[..usize::from(op.plan.normal[0])]
        {
            return Err(invalid());
        }
        let kind = match port {
            Port::Projection { step: 0, .. } if op.plan.primary[0] => Kind::Projection(0),
            Port::Projection { step: 1, .. } if op.plan.primary[1] && op.plan.normal[0] => {
                Kind::Projection(1)
            }
            Port::Operation(_) if ready => Kind::Operation,
            Port::Normal(_) if ready => Kind::Result,
            _ => return Err(invalid()),
        };
        if !matches!(kind, Kind::Projection(_)) {
            if !self
                .flow
                .spend(reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize + 1)
            {
                return Err(budget());
            }
            if reports.index.operations.get(&id) != Some(&owner) {
                return Err(invalid());
            }
        }
        if op
            .types
            .inputs
            .into_iter()
            .chain([op.types.result])
            .any(|ty| {
                matches!(
                    ty,
                    Class::Other | Class::Reference(crate::hir::ReferenceMode::Exclusive)
                )
            })
        {
            return Ok(None);
        }
        Ok(Some(Stage {
            point: id,
            owner,
            inputs: op.inputs,
            op: symbol,
            types: op.types,
            plan: op.plan,
            control: op.control,
            kind,
        }))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod reports;

#[cfg(test)]
mod limits;

#[cfg(test)]
mod equality;

#[cfg(test)]
mod equality_boundaries;

#[cfg(test)]
mod equality_limits;
