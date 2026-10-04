use super::*;
use crate::check::dependencies::Field;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Operation,
    Result,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stage {
    pub(super) point: PointId,
    pub(super) owner: usize,
    pub(super) input: PointId,
    pub(super) index: usize,
    pub(super) load: bool,
    pub(super) normal: bool,
    pub(super) control: bool,
    pub(super) kind: Kind,
}

impl Checker {
    pub(in super::super) fn validate_field_report(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        effect: &Effect,
        span: Span,
    ) -> Result<()> {
        let invalid = || Diagnostic::unsupported("proof field-effect identity mismatch", span);
        let field = self.checked_field_effect(reports, id, owner, span)?;
        let Effect::Field {
            input,
            index,
            load,
            normal,
            control,
            operation,
            result,
        } = effect
        else {
            return Err(invalid());
        };
        if *input != field.input
            || *index != field.index
            || *load != field.load
            || *normal != field.normal
            || *control != field.control
            || (!operation && !result)
            || (*result && !normal)
        {
            return Err(invalid());
        }
        Ok(())
    }

    pub(super) fn field_effect_stage(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<Option<Stage>> {
        let (id, kind) = match port {
            Port::Operation(id) => (id, Kind::Operation),
            Port::Normal(id) => (id, Kind::Result),
            _ => return Ok(None),
        };
        if !self
            .flow
            .spend(self.fields.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(Diagnostic::unsupported(
                "proof field-effect budget exhausted",
                span,
            ));
        }
        if !self.fields.contains_key(&id) {
            return Ok(None);
        }
        let field = self.checked_field_effect(reports, id, owner, span)?;
        if kind == Kind::Result && !field.normal {
            return Err(Diagnostic::unsupported(
                "proof field-effect identity mismatch",
                span,
            ));
        }
        Ok(Some(Stage {
            point: id,
            owner,
            input: field.input,
            index: field.index,
            load: field.load,
            normal: field.normal,
            control: field.control,
            kind,
        }))
    }

    pub(super) fn record_field_effect(
        &mut self,
        stage: Stage,
        effects: &mut Effects,
        limit: usize,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof field-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof field-effect identity mismatch", span);
        if !self
            .flow
            .spend(effects.len().checked_ilog2().unwrap_or(0) as usize * 2 + 9)
        {
            return Err(budget());
        }
        if stage.kind == Kind::Result && !stage.normal {
            return Err(invalid());
        }
        if let Some((owner, prior)) = effects.get(&stage.point) {
            let Effect::Field {
                input,
                index,
                load,
                normal,
                control,
                ..
            } = prior
            else {
                return Err(invalid());
            };
            if *owner != stage.owner
                || *input != stage.input
                || *index != stage.index
                || *load != stage.load
                || *normal != stage.normal
                || *control != stage.control
            {
                return Err(invalid());
            }
        } else if effects.len() >= limit {
            return Err(budget());
        }
        let (
            _,
            Effect::Field {
                operation, result, ..
            },
        ) = effects.entry(stage.point).or_insert_with(|| {
            (
                stage.owner,
                Effect::Field {
                    input: stage.input,
                    index: stage.index,
                    load: stage.load,
                    normal: stage.normal,
                    control: stage.control,
                    operation: false,
                    result: false,
                },
            )
        })
        else {
            unreachable!()
        };
        match stage.kind {
            Kind::Operation => *operation = true,
            Kind::Result => *result = true,
        }
        Ok(())
    }

    pub(super) fn checked_field_effect(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        span: Span,
    ) -> Result<&Field> {
        let budget = || Diagnostic::unsupported("proof field-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof field-effect identity mismatch", span);
        if !self.flow.spend(
            self.fields.len().checked_ilog2().unwrap_or(0) as usize
                + reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize
                + 12,
        ) {
            return Err(budget());
        }
        let field = self.fields.get(&id).ok_or_else(invalid)?;
        let point = self.points.get(id).ok_or_else(invalid)?;
        let entry = Edge::new(Port::Entry(id), Port::Entry(field.input), Route::Next);
        let result = Edge::new(Port::Operation(id), Port::Normal(id), Route::Next);
        let stage = Port::Projection { point: id, step: 0 };
        let direct = [
            entry,
            Edge::new(Port::Normal(field.input), Port::Operation(id), Route::Next),
            result,
        ];
        let shared = [
            entry,
            Edge::new(Port::Normal(field.input), stage, Route::Next),
            Edge::new(stage, Port::Operation(id), Route::Next),
            result,
        ];
        let edges = if field.load {
            &shared[..3 + usize::from(field.normal)]
        } else {
            &direct[..2 + usize::from(field.normal)]
        };
        if field.index >= field.count
            || field.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != field.span
            || reports.index.operations.get(&id) != Some(&owner)
            || !self.points.get(field.input).is_some_and(|input| {
                input.parent == Some(id)
                    && input.owner == owner
                    && input.block == point.block
                    && input.complete
                    && matches!(input.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            })
            || field.edges.as_slice() != edges
        {
            return Err(invalid());
        }
        Ok(field)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits;
