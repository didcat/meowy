use super::{Edge, Port, Route};
use crate::{
    ast::Span,
    check::{Checker, Result, dependencies::SequenceSource},
    diagnostic::Diagnostic,
    hir::PointId,
};

impl Checker {
    pub(crate) fn matcher_endpoints(
        &mut self,
        id: PointId,
        points: Vec<Option<PointId>>,
        span: Span,
    ) -> Result<()> {
        let key = SequenceSource::Stmt(id);
        self.sequence(key, points, span)?;
        if !self
            .flow
            .spend(self.endpoints.len().checked_ilog2().unwrap_or(0) as usize + 3)
        {
            return Err(Diagnostic::unsupported(
                "proof matcher endpoint budget exhausted",
                span,
            ));
        }
        let items = &self.sequences[&key].items;
        let mut edges = Vec::new();
        if let Some(Some(first)) = items.first() {
            edges.push(Edge::new(Port::Entry(id), Port::Entry(*first), Route::Next));
        }
        if let Some(Some(last)) = items.last() {
            edges.push(Edge::new(
                Port::Normal(*last),
                Port::Normal(id),
                Route::Next,
            ));
        }
        if let Some(prior) = self.endpoints.get(&key) {
            return if *prior == edges {
                Ok(())
            } else {
                Err(Diagnostic::unsupported(
                    "proof matcher endpoint identity mismatch",
                    span,
                ))
            };
        }
        if !self.edge_room(edges.len()) {
            return Err(Diagnostic::unsupported(
                "proof matcher endpoint budget exhausted",
                span,
            ));
        }
        self.endpoint_edges += edges.len();
        self.endpoints.insert(key, edges);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::check;
    use super::*;
    use crate::ast::StmtKind;

    #[test]
    pub(crate) fn matcher_endpoints_connect_statement_roots_and_preserve_conditional_exits() {
        let checker = check("flag:=false;'out{|flag|'out.leave();after:1}");
        let (key, sequence) = checker
            .sequences
            .iter()
            .find(|(key, _)| matches!(key, SequenceSource::Stmt(_)))
            .unwrap();
        let SequenceSource::Stmt(stmt) = *key else {
            panic!()
        };
        let branch = sequence.items[0].unwrap();
        assert_eq!(
            checker.endpoints[key],
            [
                Edge::new(Port::Entry(stmt), Port::Entry(branch), Route::Next),
                Edge::new(Port::Normal(branch), Port::Normal(stmt), Route::Next),
            ]
        );
        assert_eq!(checker.points[branch].parent, Some(stmt));
        assert!(
            checker
                .scope_exits
                .values()
                .all(|exit| !matches!(exit.edge.to, Port::Normal(_)))
        );
    }

    #[test]
    pub(crate) fn matcher_endpoints_order_independent_arms_without_else_if_edges() {
        let source = "|true|1;|false|2";
        let mut block = crate::parser::parse(source).unwrap();
        let other = block.stmts.pop().unwrap();
        let mut stmt = block.stmts.remove(0);
        let StmtKind::Match { arms: later } = other.kind else {
            panic!()
        };
        let StmtKind::Match { arms } = &mut stmt.kind else {
            panic!()
        };
        arms.extend(later);
        stmt.span = Span::new(0, source.len());
        let mut checker = Checker::new();
        let (root, _) = checker.checked_stmt(&stmt).unwrap();
        let sequence = &checker.sequences[&SequenceSource::Stmt(root)];
        let first = sequence.items[0].unwrap();
        let second = sequence.items[1].unwrap();
        assert_eq!(
            sequence.edges,
            [Edge::new(
                Port::Normal(first),
                Port::Entry(second),
                Route::Next
            )]
        );
        assert!(
            !checker.branch_edges[&first]
                .iter()
                .any(|edge| edge.route == Route::False && edge.to == Port::Entry(second))
        );
    }

    #[test]
    pub(crate) fn matcher_endpoints_preserve_errors_and_bound_republication() {
        let mut checker = Checker::new();
        let block = crate::parser::parse("|1|0").unwrap();
        assert_eq!(checker.block(&block, None, None).unwrap_err().code, "E215");
        assert!(
            !checker
                .endpoints
                .keys()
                .any(|key| matches!(key, SequenceSource::Stmt(_)))
        );
        let mut checker = check("|true|1");
        let (&key, sequence) = checker
            .sequences
            .iter()
            .find(|(key, _)| matches!(key, SequenceSource::Stmt(_)))
            .unwrap();
        let SequenceSource::Stmt(root) = key else {
            panic!()
        };
        let items = sequence.items.clone();
        let count = checker.endpoint_edges;
        checker
            .matcher_endpoints(root, items.clone(), Span::default())
            .unwrap();
        assert_eq!(checker.endpoint_edges, count);
        checker.endpoints.remove(&key);
        checker.endpoint_edges -= 2;
        checker.sequence_edges = super::super::MAX_EDGES;
        assert!(
            checker
                .matcher_endpoints(root, items, Span::default())
                .unwrap_err()
                .message
                .contains("endpoint budget")
        );
        assert!(!checker.endpoints.contains_key(&key));
    }
}
