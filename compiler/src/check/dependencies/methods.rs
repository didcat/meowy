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
    Stopped,
    ListSize,
    StringSize,
    Add {
        item: hir::PointId,
        capacity: usize,
        length: Option<usize>,
        may_return: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Method {
    pub(crate) owner: usize,
    pub(crate) receiver: hir::PointId,
    pub(crate) kind: Kind,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn method_operation(
        &mut self,
        id: hir::PointId,
        receiver: hir::PointId,
        kind: Kind,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof collection-method budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof collection-method identity mismatch", span);
        if !self
            .flow
            .spend(self.methods.len().checked_ilog2().unwrap_or(0) as usize * 2 + 5)
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
        let item = if let Kind::Add {
            item,
            capacity,
            length,
            ..
        } = kind
        {
            if item == receiver || length.is_some_and(|length| length > capacity) {
                return Err(invalid());
            }
            Some(item)
        } else {
            None
        };
        for child in std::iter::once(receiver).chain(item) {
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
        if let Kind::Add {
            item, may_return, ..
        } = kind
        {
            edges.extend([
                Edge::new(Port::Normal(receiver), Port::Snapshot(id), Route::Next),
                Edge::new(Port::Snapshot(id), Port::Entry(item), Route::Next),
            ]);
            if may_return {
                edges.extend([
                    Edge::new(Port::Normal(item), Port::Operation(id), Route::Checked),
                    Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
                ]);
            }
        } else if kind != Kind::Stopped {
            edges.extend([
                Edge::new(Port::Normal(receiver), Port::Operation(id), Route::Next),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
            ]);
        }
        let method = Method {
            owner: self.owner,
            receiver,
            kind,
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.methods.get(&id) {
            return if *prior == method {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(method.edges.len()) {
            return Err(budget());
        }
        self.method_edges += method.edges.len();
        self.methods.insert(id, method);
        Ok(())
    }
}

#[cfg(test)]
mod add;

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
    pub(crate) fn size_methods_keep_receiver_roots_and_distinct_list_string_operations() {
        let source = "xs:[1];view:&xs;xs.size();view.size();\"é\".size()";
        crate::compile(source).unwrap();
        let checker = check(source);
        assert_eq!(checker.methods.len(), 3);
        for ((&id, method), (text, kind)) in checker.methods.iter().zip([
            ("xs", Kind::ListSize),
            ("view", Kind::ListSize),
            ("\"é\"", Kind::StringSize),
        ]) {
            let point = &checker.points[method.receiver];
            assert_eq!(&source[point.span.start..point.span.end], text);
            assert_eq!(method.kind, kind);
            assert!(method.edges.contains(&Edge::new(
                Port::Normal(method.receiver),
                Port::Operation(id),
                Route::Next
            )));
        }
    }

    #[test]
    pub(crate) fn method_receivers_preserve_nonreturning_and_original_error_boundaries() {
        let source = "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};stop().add(missing)";
        crate::compile(source).unwrap();
        let checker = check(source);
        let (&id, method) = checker.methods.first_key_value().unwrap();
        assert_eq!(method.kind, Kind::Stopped);
        assert_eq!(
            method.edges,
            [Edge::new(
                Port::Entry(id),
                Port::Entry(method.receiver),
                Route::Next
            )]
        );
        for (source, code) in [
            ("[1].size(2)", "E212"),
            ("(1).size()", "E201"),
            ("\"x\".add(1)", "E201"),
        ] {
            assert_eq!(crate::compile(source).unwrap_err()[0].code, code);
        }
    }

    #[test]
    pub(crate) fn method_identity_and_shared_edge_budget_failures_are_atomic() {
        let mut checker = check("xs:[1];xs.size()");
        let (&id, method) = checker.methods.first_key_value().unwrap();
        let method = method.clone();
        let count = checker.method_edges;
        checker
            .method_operation(id, method.receiver, method.kind, method.span)
            .unwrap();
        assert_eq!(checker.method_edges, count);
        checker.points[method.receiver].parent = None;
        assert!(
            checker
                .method_operation(id, method.receiver, method.kind, method.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        checker.points[method.receiver].parent = Some(id);
        checker.methods.clear();
        checker.method_edges = 0;
        checker.index_edges = super::super::edges::MAX_EDGES;
        assert!(
            checker
                .method_operation(id, method.receiver, method.kind, method.span)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert!(checker.methods.is_empty());
        assert_eq!(checker.method_edges, 0);
    }
}
