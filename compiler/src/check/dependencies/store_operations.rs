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
pub(crate) struct Operation {
    pub(crate) owner: usize,
    pub(crate) target: hir::PointId,
    pub(crate) input: hir::PointId,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: [Edge; 5],
}

impl Checker {
    pub(crate) fn store_operation(
        &mut self,
        id: hir::PointId,
        target: hir::PointId,
        input: hir::PointId,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof indirect-store budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof indirect-store identity mismatch", span);
        if !self
            .flow
            .spend(self.stores.len().checked_ilog2().unwrap_or(0) as usize * 2 + 4)
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if point.kind != PointKind::Stmt
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || target == input
        {
            return Err(invalid());
        }
        for child in [target, input] {
            if !self.points.get(child).is_some_and(|source| {
                source.parent == Some(id)
                    && source.owner == self.owner
                    && source.block == point.block
                    && source.complete
                    && matches!(
                        source.kind,
                        PointKind::Expr | PointKind::And | PointKind::Or
                    )
            }) {
                return Err(invalid());
            }
        }
        let address = Port::Address { point: id, step: 0 };
        let operation = Operation {
            owner: self.owner,
            target,
            input,
            control: self.control,
            span,
            edges: [
                Edge::new(Port::Entry(id), Port::Entry(target), Route::Next),
                Edge::new(Port::Normal(target), address, Route::Next),
                Edge::new(address, Port::Entry(input), Route::Next),
                Edge::new(Port::Normal(input), Port::Operation(id), Route::Next),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
            ],
        };
        if let Some(prior) = self.stores.get(&id) {
            return if *prior == operation {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(operation.edges.len()) {
            return Err(budget());
        }
        self.store_edges += operation.edges.len();
        self.stores.insert(id, operation);
        Ok(())
    }
}

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
    pub(crate) fn stores_capture_targets_before_rhs_and_publish_effects_after_completion() {
        let source = "x:=1;y:=2;p:=&!x;*p={p=&!y;->3};*p=4";
        crate::compile(source).unwrap();
        let checker = check(source);
        assert_eq!(checker.stores.len(), 2);
        for (&id, op) in &checker.stores {
            assert_ne!(op.target, op.input);
            assert_eq!(checker.points[op.target].parent, Some(id));
            assert_eq!(checker.points[op.input].parent, Some(id));
            assert_eq!(
                &source[checker.points[op.target].span.start..checker.points[op.target].span.end],
                "p"
            );
            assert_eq!(
                op.edges,
                [
                    Edge::new(Port::Entry(id), Port::Entry(op.target), Route::Next),
                    Edge::new(
                        Port::Normal(op.target),
                        Port::Address { point: id, step: 0 },
                        Route::Next
                    ),
                    Edge::new(
                        Port::Address { point: id, step: 0 },
                        Port::Entry(op.input),
                        Route::Next
                    ),
                    Edge::new(Port::Normal(op.input), Port::Operation(id), Route::Next),
                    Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
                ]
            );
        }
    }

    #[test]
    pub(crate) fn stores_preserve_owner_control_and_original_diagnostics() {
        let mut checker = Checker::new();
        checker.derived.insert(0);
        let source = "flag:false;x:=1;p:&!x;|flag|*p=2;f:(){y:=3;q:&!y;*q=4}";
        checker
            .block(&crate::parser::parse(source).unwrap(), None, None)
            .unwrap();
        assert!(checker.stores.values().any(|op| op.control));
        assert!(checker.stores.values().any(|op| op.owner != 0));
        for (source, code) in [
            ("x:=1;p:&x;*p=2", "E305"),
            ("x:=1;p:&!x;*p=false", "E207"),
            ("x:=[1];p:&!x;*p=[2]", "B001"),
        ] {
            let mut checker = Checker::new();
            assert_eq!(
                checker
                    .block(&crate::parser::parse(source).unwrap(), None, None)
                    .unwrap_err()
                    .code,
                code
            );
            assert!(checker.stores.is_empty());
        }
        assert_eq!(
            crate::compile("x:=1;p:&!x;*p={x=2;->3}").unwrap_err()[0].code,
            "E302"
        );
    }

    #[test]
    pub(crate) fn stores_validate_child_identity_and_shared_edge_limits_atomically() {
        let mut checker = check("x:=1;p:&!x;*p=2");
        let (&id, op) = checker.stores.first_key_value().unwrap();
        let op = op.clone();
        let count = checker.store_edges;
        checker
            .store_operation(id, op.target, op.input, op.span)
            .unwrap();
        assert_eq!(checker.store_edges, count);
        for child in [op.target, op.input] {
            checker.points[child].parent = None;
            assert!(
                checker
                    .store_operation(id, op.target, op.input, op.span)
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
            checker.points[child].parent = Some(id);
            assert_eq!(checker.store_edges, count);
        }
        checker.stores.clear();
        checker.store_edges = 0;
        assert!(
            checker
                .store_operation(id, op.input, op.input, op.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        checker.sequence_edges = super::super::edges::MAX_EDGES;
        assert!(
            checker
                .store_operation(id, op.target, op.input, op.span)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert!(checker.stores.is_empty());
        assert_eq!(checker.store_edges, 0);
    }
}
