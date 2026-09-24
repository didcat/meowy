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

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ScopeExit {
    pub(crate) owner: usize,
    pub(crate) span: Span,
    pub(crate) control: bool,
    pub(crate) edge: Edge,
}

impl Checker {
    pub(crate) fn scope_exit(
        &mut self,
        id: hir::PointId,
        target: hir::BlockId,
        restart: Option<hir::RestartId>,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof scope-exit budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof scope-exit identity mismatch", span);
        if !self.flow.spend(
            self.frames.len().saturating_mul(2)
                + self.scope_exits.len().checked_ilog2().unwrap_or(0) as usize * 2
                + self.restart_edges.len().checked_ilog2().unwrap_or(0) as usize * 2
                + self.restart_inputs.len().checked_ilog2().unwrap_or(0) as usize
                + 3,
        ) {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if point.kind != PointKind::Stmt
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || !self
                .frames
                .iter()
                .any(|frame| frame.id == target && frame.owner == self.owner)
            || point.block
                != self
                    .frames
                    .iter()
                    .rev()
                    .find(|frame| frame.owner == self.owner)
                    .map(|frame| frame.id)
        {
            return Err(invalid());
        }
        let to = match restart {
            Some(site) => {
                if !self.restart_inputs.get(&site).is_some_and(|input| {
                    input.target == target && input.owner == self.owner && input.point == Some(id)
                }) {
                    return Err(invalid());
                }
                Port::Restart { target, site }
            }
            None => Port::Leave(target),
        };
        let exit = ScopeExit {
            owner: self.owner,
            span,
            control: self.control,
            edge: Edge::new(Port::Entry(id), to, Route::Exit),
        };
        if self
            .scope_exits
            .get(&id)
            .is_some_and(|prior| *prior != exit)
        {
            return Err(invalid());
        }
        let reentry = restart.map(|site| {
            (
                site,
                Edge::new(to, Port::BlockEntry(target), Route::Backedge),
            )
        });
        if let Some((site, edge)) = reentry
            && self
                .restart_edges
                .get(&site)
                .is_some_and(|prior| *prior != edge)
        {
            return Err(invalid());
        }
        let count = usize::from(!self.scope_exits.contains_key(&id))
            + usize::from(reentry.is_some_and(|(site, _)| !self.restart_edges.contains_key(&site)));
        if !self.edge_room(count) {
            return Err(budget());
        }
        self.scope_exits.entry(id).or_insert(exit);
        if let Some((site, edge)) = reentry {
            self.restart_edges.entry(site).or_insert(edge);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(crate) fn restart_edges_preserve_nested_targets_and_function_sites() {
        let source = "flag:=false;'out{'inner{|flag|'out.restart();|flag|'inner.restart()}};f:(){'loop{|false|'loop.restart()}}";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        let block = crate::parser::parse(source).unwrap();
        checker.block(&block, None, None).unwrap();
        assert_eq!(checker.restart_edges.len(), 3);
        for (site, edge) in &checker.restart_edges {
            let input = &checker.restart_inputs[site];
            let point = input.point.unwrap();
            assert_eq!(
                *edge,
                Edge::new(
                    Port::Restart {
                        target: input.target,
                        site: *site
                    },
                    Port::BlockEntry(input.target),
                    Route::Backedge
                )
            );
            assert_eq!(checker.scope_exits[&point].edge.to, edge.from);
            assert_eq!(checker.bodies[&input.target].owner, input.owner);
        }
    }

    #[test]
    pub(crate) fn restart_edges_form_explicit_backedges_without_normal_completion() {
        let mut checker = Checker::new();
        let block = crate::parser::parse("'loop{'loop.restart()}").unwrap();
        checker.block(&block, None, None).unwrap();
        let input = &checker.restart_inputs[&0];
        let point = input.point.unwrap();
        let key = super::super::SequenceSource::Block(input.target);
        assert!(checker.endpoints[&key].contains(&Edge::new(
            Port::BlockEntry(input.target),
            Port::Entry(point),
            Route::Next
        )));
        assert_eq!(
            checker.scope_exits[&point].edge.to,
            checker.restart_edges[&0].from
        );
        assert_eq!(checker.restart_edges[&0].to, Port::BlockEntry(input.target));
        assert!(
            !checker.endpoints[&key]
                .iter()
                .any(|edge| edge.to == Port::BlockResult(input.target))
        );
    }

    #[test]
    pub(crate) fn restart_edges_publish_both_edges_atomically_and_reject_source_mismatches() {
        let mut checker = Checker::new();
        let ast = crate::parser::parse("'loop{'loop.restart()}").unwrap();
        let crate::ast::StmtKind::Expr(expr) = &ast.stmts[0].kind else {
            panic!()
        };
        let crate::ast::ExprKind::Block(block) = &expr.kind else {
            panic!()
        };
        checker.block_start(block, None, None, false).unwrap();
        let (point, _) = checker.checked_stmt(&block.stmts[0]).unwrap();
        let target = checker.restart_inputs[&0].target;
        let span = checker.restart_inputs[&0].span;
        checker.scope_exits.clear();
        checker.restart_edges.clear();
        checker.sequence_edges = super::super::edges::MAX_EDGES - 1;
        assert!(
            checker
                .scope_exit(point, target, Some(0), span)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert!(checker.scope_exits.is_empty() && checker.restart_edges.is_empty());
        checker.sequence_edges -= 1;
        checker.scope_exit(point, target, Some(0), span).unwrap();
        checker.scope_exit(point, target, Some(0), span).unwrap();
        assert_eq!(checker.scope_exits.len(), 1);
        assert_eq!(checker.restart_edges.len(), 1);
        assert!(checker.edge_room(0));
        assert!(!checker.edge_room(1));
        checker.restart_inputs.get_mut(&0).unwrap().point = None;
        assert!(
            checker
                .scope_exit(point, target, Some(0), span)
                .unwrap_err()
                .message
                .contains("identity mismatch")
        );
        assert_eq!(checker.restart_edges.len(), 1);
    }

    #[test]
    pub(crate) fn scope_exits_preserve_alias_targets_restart_sites_and_derived_control() {
        let source = "flag:false;'out{stop:'out.leave;|flag|stop();'loop{|flag|'loop.restart()}}";
        crate::compile(source).unwrap();
        for derived in [false, true] {
            let mut checker = Checker::new();
            if derived {
                checker.derived.insert(0);
            }
            let block = crate::parser::parse(source).unwrap();
            checker.block(&block, None, None).unwrap();
            assert_eq!(checker.scope_exits.len(), 2);
            for (id, exit) in &checker.scope_exits {
                assert_eq!(exit.edge.from, Port::Entry(*id));
                assert_eq!(exit.edge.route, Route::Exit);
                assert_eq!(exit.control, derived);
                assert!(checker.points[*id].complete);
                match exit.edge.to {
                    Port::Leave(target) => {
                        assert_eq!(&source[exit.span.start..exit.span.end], "stop()");
                        assert_eq!(checker.points[*id].block, Some(target));
                    }
                    Port::Restart { target, site } => {
                        assert_eq!(checker.restart_inputs[&site].target, target);
                        assert_eq!(checker.restart_inputs[&site].owner, exit.owner);
                    }
                    _ => panic!(),
                }
            }
        }
    }

    #[test]
    pub(crate) fn scope_exits_keep_nested_and_function_targets_without_normal_fallthrough() {
        let source = "f:(){'out{'inner{'out.leave()}}};'out{'out.leave()}";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        let block = crate::parser::parse(source).unwrap();
        checker.block(&block, None, None).unwrap();
        let exits = checker.scope_exits.values().collect::<Vec<_>>();
        assert_eq!(exits.len(), 2);
        assert_ne!(exits[0].owner, exits[1].owner);
        let Port::Leave(target) = exits[0].edge.to else {
            panic!()
        };
        let Port::Entry(id) = exits[0].edge.from else {
            panic!()
        };
        assert_ne!(checker.points[id].block, Some(target));
        assert!(
            exits
                .iter()
                .all(|exit| !matches!(exit.edge.to, Port::Normal(_)))
        );
    }

    #[test]
    pub(crate) fn scope_exits_preserve_scope_errors_without_recording_edges() {
        for (source, code) in [
            ("'out{'out.leave(1)}", "E212"),
            ("'out{f:(){'out.leave()}}", "E201"),
            ("'out{'out.restart(1)}", "E212"),
        ] {
            let mut checker = Checker::new();
            let block = crate::parser::parse(source).unwrap();
            assert_eq!(checker.block(&block, None, None).unwrap_err().code, code);
            assert!(checker.scope_exits.is_empty());
        }
    }

    #[test]
    pub(crate) fn scope_exits_enforce_shared_capacity_and_stable_identity() {
        for capacity in [false, true] {
            let mut checker = Checker::new();
            let block = crate::parser::parse("'out{'out.leave()}").unwrap();
            let crate::ast::StmtKind::Expr(expr) = &block.stmts[0].kind else {
                panic!()
            };
            let crate::ast::ExprKind::Block(block) = &expr.kind else {
                panic!()
            };
            checker.block_start(block, None, None, false).unwrap();
            let (id, _) = checker.checked_stmt(&block.stmts[0]).unwrap();
            let span = checker.scope_exits[&id].span;
            let Port::Leave(target) = checker.scope_exits[&id].edge.to else {
                panic!()
            };
            checker.scope_exit(id, target, None, span).unwrap();
            assert!(checker.scope_exit(id, usize::MAX, None, span).is_err());
            assert_eq!(checker.scope_exits.len(), 1);
            checker.scope_exits.clear();
            if capacity {
                checker.sequence_edges = super::super::edges::MAX_EDGES;
            } else {
                assert!(!checker.flow.spend(usize::MAX));
            }
            let error = checker.scope_exit(id, target, None, span).unwrap_err();
            assert_eq!(error.code, "B001");
            assert!(error.message.contains("scope-exit budget"));
            assert!(checker.scope_exits.is_empty());
        }
    }
}
