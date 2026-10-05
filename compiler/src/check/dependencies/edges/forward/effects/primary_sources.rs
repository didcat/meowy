use super::*;

impl Checker {
    pub(in super::super) fn primary_effect_inputs(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        effect: &Effect,
        span: Span,
    ) -> Result<[Option<PointId>; 2]> {
        let budget = || Diagnostic::unsupported("proof primary-source budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof primary-source identity mismatch", span);
        let mut inputs = [None; 2];
        match effect {
            Effect::Coercion(op) => {
                if !op.projected && !op.operation && !op.result {
                    return Err(invalid());
                }
                for (port, seen) in [
                    (Port::Projection { point: id, step: 0 }, op.projected),
                    (Port::Operation(id), op.operation),
                    (Port::Normal(id), op.result),
                ] {
                    if !seen {
                        continue;
                    }
                    if !self.flow.spend(8) {
                        return Err(budget());
                    }
                    let stage = self
                        .coercion_effect_stage(reports, owner, port, span)?
                        .ok_or_else(invalid)?;
                    if op.input != stage.input
                        || op.op != stage.op
                        || op.primary != stage.primary
                        || op.source != stage.source
                        || op.control != stage.control
                    {
                        return Err(invalid());
                    }
                }
                if op.projected {
                    inputs[0] = Some(op.input);
                }
            }
            Effect::Unary(op) => {
                if !op.projected && !op.operation && !op.result {
                    return Err(invalid());
                }
                for (port, seen) in [
                    (Port::Projection { point: id, step: 0 }, op.projected),
                    (Port::Operation(id), op.operation),
                    (Port::Normal(id), op.result),
                ] {
                    if !seen {
                        continue;
                    }
                    if !self.flow.spend(9) {
                        return Err(budget());
                    }
                    let stage = self
                        .unary_effect_stage(reports, owner, port, span)?
                        .ok_or_else(invalid)?;
                    if op.input != stage.input
                        || op.op != stage.op
                        || op.ty != stage.ty
                        || op.primary != stage.primary
                        || op.checked != stage.checked
                        || op.control != stage.control
                    {
                        return Err(invalid());
                    }
                }
                if op.projected {
                    inputs[0] = Some(op.input);
                }
            }
            Effect::Binary(op) => {
                if !op.projected.into_iter().any(|seen| seen) && !op.operation && !op.result {
                    return Err(invalid());
                }
                for (port, seen) in [
                    (Port::Projection { point: id, step: 0 }, op.projected[0]),
                    (Port::Projection { point: id, step: 1 }, op.projected[1]),
                    (Port::Operation(id), op.operation),
                    (Port::Normal(id), op.result),
                ] {
                    if !seen {
                        continue;
                    }
                    if !self.flow.spend(9) {
                        return Err(budget());
                    }
                    let stage = self
                        .binary_effect_stage(reports, owner, port, span)?
                        .ok_or_else(invalid)?;
                    if op.inputs != stage.inputs
                        || op.op != stage.op
                        || op.types != stage.types
                        || op.plan != stage.plan
                        || op.control != stage.control
                    {
                        return Err(invalid());
                    }
                }
                for (step, seen) in op.projected.into_iter().enumerate() {
                    if seen {
                        inputs[step] = Some(op.inputs[step]);
                    }
                }
            }
            _ => (),
        }
        Ok(inputs)
    }
}
