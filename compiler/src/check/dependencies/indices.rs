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
pub(crate) struct Access {
    pub(crate) position: hir::PointId,
    pub(crate) capacity: usize,
    pub(crate) length: Option<usize>,
    pub(crate) may_return: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Index {
    pub(crate) owner: usize,
    pub(crate) receiver: hir::PointId,
    pub(crate) access: Option<Access>,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn index_operation(
        &mut self,
        id: hir::PointId,
        receiver: hir::PointId,
        access: Option<Access>,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof list-index budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof list-index identity mismatch", span);
        if !self
            .flow
            .spend(self.indices.len().checked_ilog2().unwrap_or(0) as usize * 2 + 5)
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if point.kind != PointKind::Expr
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
        {
            return Err(invalid());
        }
        for child in std::iter::once(receiver).chain(access.map(|access| access.position)) {
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
        let mut edges = vec![Edge::new(
            Port::Entry(id),
            Port::Entry(receiver),
            Route::Next,
        )];
        if let Some(access) = access {
            if access.position == receiver
                || access.length.is_some_and(|length| length > access.capacity)
            {
                return Err(invalid());
            }
            edges.extend([
                Edge::new(Port::Normal(receiver), Port::Snapshot(id), Route::Next),
                Edge::new(
                    Port::Snapshot(id),
                    Port::Entry(access.position),
                    Route::Next,
                ),
            ]);
            if access.may_return {
                edges.extend([
                    Edge::new(
                        Port::Normal(access.position),
                        Port::Operation(id),
                        Route::Checked,
                    ),
                    Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
                ]);
            }
        }
        let index = Index {
            owner: self.owner,
            receiver,
            access,
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.indices.get(&id) {
            return if *prior == index {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(index.edges.len()) {
            return Err(budget());
        }
        self.index_edges += index.edges.len();
        self.indices.insert(id, index);
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
    pub(crate) fn indices_snapshot_values_and_lengths_before_side_effectful_positions() {
        for (source, text, length) in [
            ("xs<int32[3]>:=[1,2];y:xs[{xs=[];->1}]", "xs", None),
            (
                "xs<int32[3]>:=[1,2];view:&xs;y:view[{xs=[];->1}]",
                "view",
                None,
            ),
            ("xs<int32[3]>:[1,2];y:xs[1]", "xs", Some(2)),
        ] {
            crate::compile(source).unwrap();
            let checker = check(source);
            let (&id, index) = checker.indices.first_key_value().unwrap();
            let access = index.access.unwrap();
            assert_eq!(access.capacity, 3);
            let receiver = &checker.points[index.receiver];
            assert_eq!(&source[receiver.span.start..receiver.span.end], text);
            assert_eq!(
                index.edges,
                [
                    Edge::new(Port::Entry(id), Port::Entry(index.receiver), Route::Next),
                    Edge::new(
                        Port::Normal(index.receiver),
                        Port::Snapshot(id),
                        Route::Next
                    ),
                    Edge::new(
                        Port::Snapshot(id),
                        Port::Entry(access.position),
                        Route::Next
                    ),
                    Edge::new(
                        Port::Normal(access.position),
                        Port::Operation(id),
                        Route::Checked
                    ),
                    Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
                ]
            );
            assert_eq!(access.length, length);
        }
    }

    #[test]
    pub(crate) fn indices_preserve_nested_owners_and_nonreturning_operand_boundaries() {
        let source = "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};f<int32>:(xs<int32[2][2]>){->xs[1][1]};stop()[missing]";
        crate::compile(source).unwrap();
        let checker = check(source);
        assert_eq!(checker.indices.len(), 3);
        assert_eq!(
            checker
                .indices
                .values()
                .filter(|index| index.owner != 0)
                .count(),
            2
        );
        let (&id, stopped) = checker
            .indices
            .iter()
            .find(|(_, index)| index.access.is_none())
            .unwrap();
        assert_eq!(
            stopped.edges,
            [Edge::new(
                Port::Entry(id),
                Port::Entry(stopped.receiver),
                Route::Next
            )]
        );
        let source = "xs:[1];'out{y:xs[{'out.leave()}]}";
        crate::compile(source).unwrap();
        let checker = check(source);
        let (&id, index) = checker.indices.first_key_value().unwrap();
        assert!(!index.access.unwrap().may_return);
        assert!(!index.edges.iter().any(|edge| edge.to == Port::Normal(id)));
        assert_eq!(checker.scope_exits.len(), 1);
    }

    #[test]
    pub(crate) fn indices_preserve_one_based_bounds_types_and_live_borrows() {
        for (source, code) in [
            ("xs<int32[3]>:[1];xs[2]", "E101"),
            ("xs:[1];xs[0]", "E101"),
            ("xs:[1];xs[false]", "E222"),
            ("xs:1;xs[1]", "B001"),
            ("xs:=[1];p:&!xs;xs[1];after:*p", "B001"),
            ("xs:=[1];p:&!(xs[1]);xs[1];after:*p", "E302"),
        ] {
            assert_eq!(
                crate::compile(source).unwrap_err()[0].code,
                code,
                "{source}"
            );
        }
    }

    #[test]
    pub(crate) fn indices_validate_roots_lengths_and_shared_edge_budgets_atomically() {
        let mut checker = check("xs:[1];xs[1]");
        let (&id, index) = checker.indices.first_key_value().unwrap();
        let index = index.clone();
        let count = checker.index_edges;
        checker
            .index_operation(id, index.receiver, index.access, index.span)
            .unwrap();
        assert_eq!(checker.index_edges, count);
        checker.indices.clear();
        checker.index_edges = 0;
        let mut access = index.access.unwrap();
        access.position = index.receiver;
        assert!(
            checker
                .index_operation(id, index.receiver, Some(access), index.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        access = index.access.unwrap();
        access.length = Some(access.capacity + 1);
        assert!(
            checker
                .index_operation(id, index.receiver, Some(access), index.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        checker.sequence_edges = super::super::edges::MAX_EDGES;
        assert!(
            checker
                .index_operation(id, index.receiver, index.access, index.span)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert!(checker.indices.is_empty());
        assert_eq!(checker.index_edges, 0);
    }
}
