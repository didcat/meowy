use super::{Edge, Port, Route};
use crate::{
    ast::Span,
    check::{Checker, Result, dependencies::SequenceSource},
    diagnostic::Diagnostic,
    hir,
};

impl Checker {
    pub(crate) fn block_endpoints(&mut self, body: &hir::Block, span: Span) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof block endpoint budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof block endpoint identity mismatch", span);
        if !self.flow.spend(
            self.sequences.len().checked_ilog2().unwrap_or(0) as usize
                + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                + self.endpoints.len().checked_ilog2().unwrap_or(0) as usize
                + 5,
        ) {
            return Err(budget());
        }
        let key = SequenceSource::Block(body.id);
        let sequence = self.sequences.get(&key).ok_or_else(invalid)?;
        if sequence.owner != self.owner
            || !self
                .bodies
                .get(&body.id)
                .is_some_and(|body| body.owner == self.owner)
        {
            return Err(invalid());
        }
        let mut edges = vec![Edge::new(
            Port::Leave(body.id),
            Port::BlockNormal(body.id),
            Route::Join,
        )];
        if sequence.items.is_empty() {
            edges.push(Edge::new(
                Port::BlockEntry(body.id),
                Port::BlockNormal(body.id),
                Route::Next,
            ));
        } else {
            if let Some(Some(first)) = sequence.items.first() {
                edges.push(Edge::new(
                    Port::BlockEntry(body.id),
                    Port::Entry(*first),
                    Route::Next,
                ));
            }
            if let Some(Some(last)) = sequence.items.last() {
                edges.push(Edge::new(
                    Port::Normal(*last),
                    Port::BlockNormal(body.id),
                    Route::Next,
                ));
            }
        }
        if body.ty != hir::Type::Never {
            edges.push(Edge::new(
                Port::BlockNormal(body.id),
                Port::BlockResult(body.id),
                Route::Result,
            ));
        }
        if let Some(prior) = self.endpoints.get(&key) {
            return if *prior == edges {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(edges.len()) {
            return Err(budget());
        }
        self.endpoint_edges += edges.len();
        self.endpoints.insert(key, edges);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn check(source: &str) -> (Checker, hir::Block) {
        let mut checker = Checker::new();
        let block = crate::parser::parse(source).unwrap();
        let body = checker.block(&block, None, None).unwrap();
        (checker, body)
    }

    #[test]
    pub(crate) fn block_endpoints_connect_ordered_statements_and_empty_results() {
        let (checker, body) = check("a:1;b:2");
        let key = SequenceSource::Block(body.id);
        let items = &checker.sequences[&key].items;
        let edges = &checker.endpoints[&key];
        assert!(edges.contains(&Edge::new(
            Port::BlockEntry(body.id),
            Port::Entry(items[0].unwrap()),
            Route::Next
        )));
        assert!(edges.contains(&Edge::new(
            Port::Normal(items[1].unwrap()),
            Port::BlockNormal(body.id),
            Route::Next
        )));
        let (checker, body) = check("");
        let edges = &checker.endpoints[&SequenceSource::Block(body.id)];
        assert!(edges.contains(&Edge::new(
            Port::BlockEntry(body.id),
            Port::BlockNormal(body.id),
            Route::Next
        )));
        assert!(edges.contains(&Edge::new(
            Port::BlockNormal(body.id),
            Port::BlockResult(body.id),
            Route::Result
        )));
    }

    #[test]
    pub(crate) fn block_endpoints_do_not_skip_forward_or_receiver_prefixes() {
        for source in ["f<()->int32>;f<int32>:(){->1};x:2", "x:3.{->$}"] {
            crate::compile(source).unwrap();
            let (checker, _) = check(source);
            let mut found = false;
            for (key, sequence) in &checker.sequences {
                let SequenceSource::Block(block) = key else {
                    continue;
                };
                if sequence.items.first() == Some(&None) {
                    found = true;
                    assert!(
                        !checker.endpoints[key]
                            .iter()
                            .any(|edge| edge.from == Port::BlockEntry(*block))
                    );
                }
            }
            assert!(found);
        }
    }

    #[test]
    pub(crate) fn block_endpoints_keep_target_leaves_and_never_results_distinct() {
        let (checker, _) = check("'out{'inner{'out.leave()}};'loop{'loop.restart()}");
        for exit in checker.scope_exits.values() {
            match exit.edge.to {
                Port::Leave(target) => assert!(
                    checker.endpoints[&SequenceSource::Block(target)].contains(&Edge::new(
                        Port::Leave(target),
                        Port::BlockNormal(target),
                        Route::Join
                    ))
                ),
                Port::Restart { target, .. } => assert!(
                    !checker.endpoints[&SequenceSource::Block(target)]
                        .iter()
                        .any(|edge| edge.to == Port::BlockResult(target))
                ),
                _ => panic!(),
            }
        }
    }

    #[test]
    pub(crate) fn block_endpoints_bound_atomic_registration_and_identity() {
        let (mut checker, body) = check("1");
        let count = checker.endpoint_edges;
        checker.block_endpoints(&body, Span::default()).unwrap();
        assert_eq!(checker.endpoint_edges, count);
        let mut changed = body.clone();
        changed.ty = hir::Type::Never;
        assert!(
            checker
                .block_endpoints(&changed, Span::default())
                .unwrap_err()
                .message
                .contains("identity mismatch")
        );
        checker.endpoints.clear();
        checker.endpoint_edges = 0;
        checker.sequence_edges = super::super::MAX_EDGES;
        assert!(
            checker
                .block_endpoints(&body, Span::default())
                .unwrap_err()
                .message
                .contains("endpoint budget")
        );
        assert!(checker.endpoints.is_empty());
        assert_eq!(checker.endpoint_edges, 0);
    }
}
