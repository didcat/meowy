use super::{
    SequenceSource,
    edges::{Edge, Port, Route},
};
use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Plan {
    pub(crate) primary: [bool; 2],
    pub(crate) normal: [bool; 2],
    pub(crate) checked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Binary {
    pub(crate) owner: usize,
    pub(crate) inputs: [hir::PointId; 2],
    pub(crate) op: String,
    pub(crate) plan: Plan,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn binary_operation(
        &mut self,
        id: hir::PointId,
        inputs: [hir::PointId; 2],
        plan: Plan,
        value: &hir::Expr,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof binary-operation budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof binary-operation identity mismatch", span);
        if !self
            .flow
            .spend(self.binaries.len().checked_ilog2().unwrap_or(0) as usize * 2 + 5)
        {
            return Err(budget());
        }
        let hir::ExprKind::Binary {
            op,
            left,
            right,
            point: None,
        } = &value.kind
        else {
            return Err(invalid());
        };
        let normal = [left.ty != hir::Type::Never, right.ty != hir::Type::Never];
        let ready = normal.iter().all(|value| *value);
        let checked = ready
            && matches!(left.ty, hir::Type::Int { .. })
            && matches!(op.as_str(), "+" | "-" | "*" | "/" | "%");
        if !matches!(
            op.as_str(),
            "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^" | "==" | "!=" | "<" | "<=" | ">" | ">="
        ) || plan.normal != normal
            || plan.checked != checked
            || ready != (value.ty != hir::Type::Never)
        {
            return Err(invalid());
        }
        for (primary, value) in plan.primary.iter().zip([left, right]) {
            if *primary
                && !matches!(&value.kind, hir::ExprKind::Primary(input) if matches!(input.ty, hir::Type::Record { .. }))
            {
                return Err(invalid());
            }
        }
        let key = SequenceSource::Expr(id);
        let mut sequence = self.prepare_sequence(key, inputs.map(Some).to_vec(), span)?;
        sequence.edges.clear();
        let mut edges = vec![Edge::new(
            Port::Entry(id),
            Port::Entry(inputs[0]),
            Route::Next,
        )];
        let mut from = Port::Normal(inputs[0]);
        if plan.primary[0] {
            let stage = Port::Projection { point: id, step: 0 };
            edges.push(Edge::new(from, stage, Route::Next));
            from = stage;
        }
        if normal[0] {
            sequence
                .edges
                .push(Edge::new(from, Port::Entry(inputs[1]), Route::Next));
            let mut from = Port::Normal(inputs[1]);
            if plan.primary[1] {
                let stage = Port::Projection { point: id, step: 1 };
                edges.push(Edge::new(from, stage, Route::Next));
                from = stage;
            }
            if normal[1] {
                edges.push(Edge::new(from, Port::Operation(id), Route::Next));
                let route = if checked { Route::Checked } else { Route::Next };
                edges.push(Edge::new(Port::Operation(id), Port::Normal(id), route));
            }
        }
        let binary = Binary {
            owner: self.owner,
            inputs,
            op: op.clone(),
            plan,
            control: self.control,
            span,
            edges,
        };
        match (self.binaries.get(&id), self.sequences.get(&key)) {
            (Some(prior), Some(seq)) if *prior == binary && *seq == sequence => return Ok(()),
            (None, None) => (),
            _ => return Err(invalid()),
        }
        if !self.edge_room(binary.edges.len() + sequence.edges.len()) {
            return Err(budget());
        }
        self.binary_edges += binary.edges.len();
        self.sequence_edges += sequence.edges.len();
        self.binaries.insert(id, binary);
        self.sequences.insert(key, sequence);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
