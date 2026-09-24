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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Bind,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Operation {
    pub(crate) kind: Kind,
    pub(crate) owner: usize,
    pub(crate) local: hir::LocalId,
    pub(crate) storage: hir::LocalId,
    pub(crate) input: Option<hir::PointId>,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn storage_operation(
        &mut self,
        id: hir::PointId,
        kind: Kind,
        local: hir::LocalId,
        input: Option<hir::PointId>,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof storage-operation budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof storage-operation identity mismatch", span);
        if !self.flow.spend(
            self.operations.len().checked_ilog2().unwrap_or(0) as usize * 2
                + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                + 4,
        ) {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        let storage = self
            .proofs
            .aliases
            .get(&local)
            .map_or(local, |alias| alias.root);
        if point.kind != PointKind::Stmt
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || self.locals.get(local).is_none()
            || self.locals.get(storage).is_none()
        {
            return Err(invalid());
        }
        let mut edges = Vec::new();
        if let Some(input) = input {
            if !self.points.get(input).is_some_and(|source| {
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
            edges.push(Edge::new(Port::Entry(id), Port::Entry(input), Route::Next));
            edges.push(Edge::new(
                Port::Normal(input),
                Port::Operation(id),
                Route::Next,
            ));
        }
        edges.push(Edge::new(
            Port::Operation(id),
            Port::Normal(id),
            Route::Next,
        ));
        let operation = Operation {
            kind,
            owner: self.owner,
            local,
            storage,
            input,
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.operations.get(&id) {
            return if *prior == operation {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(operation.edges.len()) {
            return Err(budget());
        }
        self.operation_edges += operation.edges.len();
        self.operations.insert(id, operation);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn check(source: &str) -> Checker {
        let mut checker = Checker::new();
        let block = crate::parser::parse(source).unwrap();
        checker.block(&block, None, None).unwrap();
        checker
    }

    #[test]
    pub(crate) fn binding_operations_separate_initializer_completion_from_storage_effects() {
        let source = "a:1;b:a;c<int32[2]>:[1,2]";
        crate::compile(source).unwrap();
        let checker = check(source);
        assert_eq!(checker.operations.len(), 3);
        for (id, operation) in &checker.operations {
            let input = operation.input.unwrap();
            assert_eq!(operation.kind, Kind::Bind);
            assert_eq!(operation.local, operation.storage);
            assert_eq!(checker.points[input].parent, Some(*id));
            assert!(operation.edges.contains(&Edge::new(
                Port::Normal(input),
                Port::Operation(*id),
                Route::Next
            )));
            assert!(!operation.edges.contains(&Edge::new(
                Port::Normal(input),
                Port::Normal(*id),
                Route::Next
            )));
        }
    }

    #[test]
    pub(crate) fn binding_operations_keep_control_and_function_owners_without_descriptor_storage() {
        let mut checker = Checker::new();
        checker.derived.insert(0);
        let source = "p:@\"proof\";flag:false;|flag|x:1;f:(){y:2};q:p.can_copy<uint8>()";
        let block = crate::parser::parse(source).unwrap();
        checker.block(&block, None, None).unwrap();
        assert_eq!(checker.operations.len(), 3);
        assert_eq!(
            checker.operations.values().filter(|op| op.control).count(),
            1
        );
        assert!(checker.operations.values().any(|op| op.owner != 0));
        assert_eq!(checker.queries.len(), 1);
    }

    #[test]
    pub(crate) fn binding_operations_retain_unknown_inputs_and_bound_atomic_publication() {
        let mut checker = check("x:1");
        let (&id, operation) = checker.operations.first_key_value().unwrap();
        let local = operation.local;
        let input = operation.input;
        let span = operation.span;
        checker.operation_edges -= operation.edges.len();
        checker.operations.clear();
        checker
            .storage_operation(id, Kind::Bind, local, None, span)
            .unwrap();
        assert!(checker.operations[&id].input.is_none());
        assert_eq!(
            checker.operations[&id].edges,
            [Edge::new(
                Port::Operation(id),
                Port::Normal(id),
                Route::Next
            )]
        );
        checker.operations.clear();
        checker.operation_edges = 0;
        checker.sequence_edges = super::super::edges::MAX_EDGES;
        assert!(
            checker
                .storage_operation(id, Kind::Bind, local, input, span)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert!(checker.operations.is_empty());
        assert_eq!(checker.operation_edges, 0);
    }

    #[test]
    pub(crate) fn binding_operations_preserve_ordinary_errors_and_reject_foreign_inputs() {
        let mut checker = Checker::new();
        let block = crate::parser::parse("x<boolean>:1").unwrap();
        assert_eq!(checker.block(&block, None, None).unwrap_err().code, "E207");
        assert!(checker.operations.is_empty());
        let mut checker = check("x:1");
        let (&id, operation) = checker.operations.first_key_value().unwrap();
        let (local, input, span) = (operation.local, operation.input, operation.span);
        let count = checker.operation_edges;
        checker
            .storage_operation(id, Kind::Bind, local, input, span)
            .unwrap();
        assert_eq!(checker.operation_edges, count);
        checker.points[input.unwrap()].parent = None;
        assert!(
            checker
                .storage_operation(id, Kind::Bind, local, input, span)
                .unwrap_err()
                .message
                .contains("identity mismatch")
        );
        assert_eq!(checker.operation_edges, count);
    }
}
