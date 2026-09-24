use super::{
    PointKind,
    edges::{Edge, Port, Route},
};
use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Field(usize),
    Index {
        point: hir::PointId,
        capacity: usize,
        span: Span,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Operation {
    pub(crate) owner: usize,
    pub(crate) local: hir::LocalId,
    pub(crate) storage: hir::LocalId,
    pub(crate) steps: Vec<Step>,
    pub(crate) input: hir::PointId,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn path_operation(
        &mut self,
        id: hir::PointId,
        local: hir::LocalId,
        steps: Vec<Step>,
        input: hir::PointId,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof path-operation budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof path-operation identity mismatch", span);
        if steps.len() > crate::list::MAX_WRITE_PATH
            || !self.flow.spend(
                self.paths.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                    + steps.len() * (steps.len().checked_ilog2().unwrap_or(0) as usize + 3)
                    + 4,
            )
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        let storage = self
            .proofs
            .aliases
            .get(&local)
            .map_or(local, |alias| alias.root);
        let child = |input: hir::PointId| {
            self.points.get(input).is_some_and(|source| {
                source.parent == Some(id)
                    && source.owner == self.owner
                    && source.block == point.block
                    && source.complete
                    && matches!(
                        source.kind,
                        PointKind::Expr | PointKind::And | PointKind::Or
                    )
            })
        };
        if point.kind != PointKind::Stmt
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || self.locals.get(storage).is_none()
            || steps.is_empty()
            || !child(input)
        {
            return Err(invalid());
        }
        let mut ty = self.locals.get(local).ok_or_else(invalid)?;
        let mut inputs = std::collections::BTreeSet::from([input]);
        let address = |step| Port::Address { point: id, step };
        let mut edges = vec![Edge::new(Port::Entry(id), address(0), Route::Next)];
        for (index, step) in steps.iter().enumerate() {
            match step {
                Step::Field(field) => {
                    let hir::Type::Record { fields, .. } = ty else {
                        return Err(invalid());
                    };
                    ty = &fields.get(*field).ok_or_else(invalid)?.ty;
                    edges.push(Edge::new(address(index), address(index + 1), Route::Next));
                }
                Step::Index {
                    point, capacity, ..
                } => {
                    let hir::Type::List {
                        element,
                        capacity: size,
                    } = ty
                    else {
                        return Err(invalid());
                    };
                    if capacity != size || !child(*point) || !inputs.insert(*point) {
                        return Err(invalid());
                    }
                    let reserve = Port::Reserve {
                        point: id,
                        step: index,
                    };
                    edges.extend([
                        Edge::new(address(index), reserve, Route::Next),
                        Edge::new(reserve, Port::Entry(*point), Route::Next),
                        Edge::new(Port::Normal(*point), address(index + 1), Route::Checked),
                    ]);
                    ty = element;
                }
            }
        }
        edges.extend([
            Edge::new(address(steps.len()), Port::Entry(input), Route::Next),
            Edge::new(Port::Normal(input), Port::Operation(id), Route::Next),
            Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
        ]);
        let operation = Operation {
            owner: self.owner,
            local,
            storage,
            steps,
            input,
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.paths.get(&id) {
            return if *prior == operation {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(operation.edges.len()) {
            return Err(budget());
        }
        self.path_edges += operation.edges.len();
        self.paths.insert(id, operation);
        Ok(())
    }
}

#[cfg(test)]
mod indexed;

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn check(source: &str) -> Checker {
        let mut checker = Checker::new();
        checker
            .block(&crate::parser::parse(source).unwrap(), None, None)
            .unwrap();
        checker
    }

    #[test]
    pub(crate) fn field_stores_capture_nested_addresses_before_rhs_and_effects_after_rhs() {
        let source = "r:={->child:={->n:=1}};r.child.n=2;r.child.n=3";
        crate::compile(source).unwrap();
        let checker = check(source);
        assert_eq!(checker.paths.len(), 2);
        let mut inputs = Vec::new();
        for (&id, op) in &checker.paths {
            assert_eq!(op.steps, [Step::Field(0), Step::Field(0)]);
            assert_eq!(op.local, op.storage);
            inputs.push(op.input);
            assert_eq!(
                op.edges,
                [
                    Edge::new(
                        Port::Entry(id),
                        Port::Address { point: id, step: 0 },
                        Route::Next
                    ),
                    Edge::new(
                        Port::Address { point: id, step: 0 },
                        Port::Address { point: id, step: 1 },
                        Route::Next
                    ),
                    Edge::new(
                        Port::Address { point: id, step: 1 },
                        Port::Address { point: id, step: 2 },
                        Route::Next
                    ),
                    Edge::new(
                        Port::Address { point: id, step: 2 },
                        Port::Entry(op.input),
                        Route::Next
                    ),
                    Edge::new(Port::Normal(op.input), Port::Operation(id), Route::Next),
                    Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
                ]
            );
        }
        assert_ne!(inputs[0], inputs[1]);
        crate::compile("r:={->n:=1;->other:=2};r.n={r={->n:=3;->other:=4};->5}").unwrap();
    }

    #[test]
    pub(crate) fn field_stores_preserve_control_owners_and_ordinary_errors() {
        let mut checker = Checker::new();
        checker.derived.insert(0);
        let source = "flag:false;r:{->n:=1};|flag|r.n=2;f:(){s:{->n:=1};s.n=3}";
        checker
            .block(&crate::parser::parse(source).unwrap(), None, None)
            .unwrap();
        assert!(checker.paths.values().any(|op| op.control));
        assert!(checker.paths.values().any(|op| op.owner != 0));
        for (source, code) in [
            ("r:{->n:1};r.n=2", "E305"),
            ("r:{->n:=1};r.n=false", "E207"),
            ("r:{->n:=1};r.missing=2", "E201"),
        ] {
            let mut checker = Checker::new();
            assert_eq!(
                checker
                    .block(&crate::parser::parse(source).unwrap(), None, None)
                    .unwrap_err()
                    .code,
                code
            );
            assert!(checker.paths.is_empty());
        }
        assert_eq!(
            crate::compile("r:{->n:=1};p:&(r.n);r.n=2;after:*p").unwrap_err()[0].code,
            "E302"
        );
    }

    #[test]
    pub(crate) fn field_stores_validate_identity_and_publish_with_shared_edge_budget() {
        let mut checker = check("r:{->n:=1};r.n=2");
        let (&id, op) = checker.paths.first_key_value().unwrap();
        let op = op.clone();
        let count = checker.path_edges;
        checker
            .path_operation(id, op.local, op.steps.clone(), op.input, op.span)
            .unwrap();
        assert_eq!(checker.path_edges, count);
        checker.points[op.input].parent = None;
        assert!(
            checker
                .path_operation(id, op.local, op.steps.clone(), op.input, op.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        checker.points[op.input].parent = Some(id);
        checker.paths.clear();
        checker.path_edges = 0;
        checker.sequence_edges = super::super::edges::MAX_EDGES;
        assert!(
            checker
                .path_operation(id, op.local, op.steps, op.input, op.span)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert!(checker.paths.is_empty());
        assert_eq!(checker.path_edges, 0);
    }
}
