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

impl Checker {
    pub(crate) fn list_sequence(
        &mut self,
        id: hir::PointId,
        items: Vec<Option<hir::PointId>>,
        normal: bool,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof list-construction budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof list-construction identity mismatch", span);
        if items.len() > super::sequences::MAX_ITEMS
            || !self.flow.spend(
                items.len()
                    + self.endpoints.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + self.sequences.len().checked_ilog2().unwrap_or(0) as usize
                    + 4,
            )
        {
            return Err(budget());
        }
        let key = SequenceSource::Expr(id);
        let mut edges = Vec::new();
        if items.is_empty() {
            edges.push(Edge::new(Port::Entry(id), Port::Operation(id), Route::Next));
        } else {
            if let Some(Some(first)) = items.first() {
                edges.push(Edge::new(Port::Entry(id), Port::Entry(*first), Route::Next));
            }
            if normal && let Some(Some(last)) = items.last() {
                edges.push(Edge::new(
                    Port::Normal(*last),
                    Port::Operation(id),
                    Route::Next,
                ));
            }
        }
        if normal {
            edges.push(Edge::new(
                Port::Operation(id),
                Port::Normal(id),
                Route::Next,
            ));
        }
        let prior = self.endpoints.get(&key);
        if prior.is_some_and(|prior| *prior != edges) {
            return Err(invalid());
        }
        let count = if self.sequences.contains_key(&key) {
            0
        } else {
            items
                .windows(2)
                .filter(|pair| pair[0].is_some() && pair[1].is_some())
                .count()
        };
        let exists = prior.is_some();
        let extra = if exists { 0 } else { edges.len() };
        if !self.edge_room(count + extra) {
            return Err(budget());
        }
        self.sequence(key, items, span)?;
        if !exists {
            self.endpoint_edges += extra;
            self.endpoints.insert(key, edges);
        }
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
    pub(crate) fn list_sequences_follow_source_slots_despite_deferred_scalar_checking() {
        let source = "n<uint8>:2;xs:[1,n,3]";
        crate::compile(source).unwrap();
        let checker = check(source);
        let (key, sequence) = checker
            .sequences
            .iter()
            .find(|(key, _)| matches!(key, SequenceSource::Expr(_)))
            .unwrap();
        let SequenceSource::Expr(id) = *key else {
            panic!()
        };
        let points: Vec<_> = sequence.items.iter().map(|id| id.unwrap()).collect();
        assert!(points[0] > points[1]);
        for (point, text) in points.iter().zip(["1", "n", "3"]) {
            let point = &checker.points[*point];
            assert_eq!(&source[point.span.start..point.span.end], text);
        }
        assert_eq!(
            sequence.edges,
            [
                Edge::new(Port::Normal(points[0]), Port::Entry(points[1]), Route::Next),
                Edge::new(Port::Normal(points[1]), Port::Entry(points[2]), Route::Next),
            ]
        );
        assert!(checker.endpoints[key].contains(&Edge::new(
            Port::Normal(points[2]),
            Port::Operation(id),
            Route::Next
        )));
    }

    #[test]
    pub(crate) fn contextual_lists_keep_empty_construction_and_nested_side_effects() {
        for source in [
            "xs<int32[3]>:[]",
            "x:=0;xs<int32[2][2]>:[[{x=1;->x}],[{x=2;->x}]]",
        ] {
            crate::compile(source).unwrap();
            let checker = check(source);
            for (key, sequence) in &checker.sequences {
                let SequenceSource::Expr(id) = *key else {
                    continue;
                };
                let edges = &checker.endpoints[key];
                assert!(edges.contains(&Edge::new(
                    Port::Operation(id),
                    Port::Normal(id),
                    Route::Next
                )));
                if sequence.items.is_empty() {
                    assert!(edges.contains(&Edge::new(
                        Port::Entry(id),
                        Port::Operation(id),
                        Route::Next
                    )));
                }
            }
        }
        for (source, code) in [
            ("xs<int32[1]>:[1,2]", "E103"),
            ("xs<uint8[1]>:[256]", "E216"),
            ("xs:[]", "E207"),
        ] {
            assert_eq!(crate::compile(source).unwrap_err()[0].code, code);
        }
    }

    #[test]
    pub(crate) fn list_sequences_do_not_publish_results_after_nonreturning_elements() {
        let source = "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};xs<int32[2]>:[stop(),1]";
        crate::compile(source).unwrap();
        let checker = check(source);
        let (key, sequence) = checker
            .sequences
            .iter()
            .find(|(key, _)| matches!(key, SequenceSource::Expr(_)))
            .unwrap();
        let SequenceSource::Expr(id) = *key else {
            panic!()
        };
        assert_eq!(sequence.items.len(), 2);
        assert!(
            !checker.endpoints[key]
                .iter()
                .any(|edge| edge.to == Port::Normal(id))
        );
    }

    #[test]
    pub(crate) fn list_sequences_publish_no_partial_edges_on_budget_or_identity_failure() {
        let mut checker = check("xs:[1,2]");
        let (&key, sequence) = checker
            .sequences
            .iter()
            .find(|(key, _)| matches!(key, SequenceSource::Expr(_)))
            .unwrap();
        let SequenceSource::Expr(id) = key else {
            panic!()
        };
        let items = sequence.items.clone();
        checker.sequence_edges -= sequence.edges.len();
        checker.sequences.remove(&key);
        checker.endpoint_edges -= checker.endpoints.remove(&key).unwrap().len();
        let mut invalid = items.clone();
        invalid[1] = invalid[0];
        assert!(
            checker
                .list_sequence(id, invalid, true, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert!(!checker.sequences.contains_key(&key));
        checker.index_edges = super::super::edges::MAX_EDGES;
        assert!(
            checker
                .list_sequence(id, items, true, Span::default())
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert!(!checker.sequences.contains_key(&key));
        assert!(!checker.endpoints.contains_key(&key));
    }
}
