use super::{Checker, Diagnostic, Result, Span};
use crate::hir;

pub(super) const MAX_QUERY_SCOPES: usize = 256;

impl Checker {
    pub(super) fn query_scopes(&mut self, span: Span) -> Result<Vec<usize>> {
        if !self.flow.spend(self.frames.len() + 1) {
            return Err(Diagnostic::unsupported(
                "pending query scope budget exhausted",
                span,
            ));
        }
        let mut scopes = Vec::new();
        for frame in &self.frames {
            if frame.owner != self.owner {
                continue;
            }
            if scopes.len() == MAX_QUERY_SCOPES {
                return Err(Diagnostic::unsupported(
                    "pending query scope capacity exhausted",
                    span,
                ));
            }
            scopes.push(frame.id);
        }
        Ok(scopes)
    }

    pub(super) fn query_restart_sites(
        &mut self,
        scopes: &[usize],
        span: Span,
    ) -> Result<Vec<hir::RestartId>> {
        if !self.flow.spend(self.restart_inputs.len() + 1) {
            return Err(Diagnostic::unsupported(
                "pending query restart budget exhausted",
                span,
            ));
        }
        let mut sites = Vec::new();
        for (site, input) in &self.restart_inputs {
            if input.owner != self.owner {
                continue;
            }
            if !self
                .flow
                .spend(scopes.len() + self.queries.len().checked_ilog2().unwrap_or(0) as usize + 2)
            {
                return Err(Diagnostic::unsupported(
                    "pending query restart budget exhausted",
                    span,
                ));
            }
            if scopes.contains(&input.target) {
                sites.push(*site);
            }
        }
        Ok(sites)
    }

    pub(crate) fn restart_query_ids(
        &mut self,
        owner: usize,
        target: usize,
        span: Span,
    ) -> Result<Vec<usize>> {
        if !self.flow.spend(
            self.queries.len() * (self.sites.len().checked_ilog2().unwrap_or(0) as usize + 1) + 1,
        ) {
            return Err(Diagnostic::unsupported(
                "restart query association budget exhausted",
                span,
            ));
        }
        let mut queries = Vec::new();
        for (id, query) in self.queries.iter().enumerate() {
            let source = query.site.map_or(self.points[query.point].owner, |site| {
                self.sites[&site].owner
            });
            if source != owner {
                continue;
            }
            if !self.flow.spend(
                query.scopes.len() + self.queries.len().checked_ilog2().unwrap_or(0) as usize + 2,
            ) {
                return Err(Diagnostic::unsupported(
                    "restart query association budget exhausted",
                    span,
                ));
            }
            if query.scopes.contains(&target) {
                queries.push(id);
            }
        }
        Ok(queries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    pub(crate) fn check(source: &str) -> Checker {
        let mut checker = Checker::new();
        let block = crate::parser::parse(source).unwrap();
        checker.block(&block, None, None).unwrap();
        checker
    }

    #[test]
    pub(crate) fn query_scopes_link_restarts_in_both_source_orders() {
        let checker = check(
            "p:@\"proof\";flag:=false;outside:p.can_copy<uint8>();'loop{before:p.can_copy<uint16>();|flag|'loop.restart();after:p.can_copy<uint32>()};done:p.can_copy<uint64>()",
        );
        assert_eq!(checker.restart_queries[&0], BTreeSet::from([1, 2]));
        let target = checker.restart_inputs[&0].target;
        assert!(!checker.queries[0].scopes.contains(&target));
        assert!(checker.queries[1].scopes.contains(&target));
        assert!(checker.queries[2].scopes.contains(&target));
        assert!(!checker.queries[3].scopes.contains(&target));
        assert!(checker.queries.iter().all(|query| !query.control));
        assert_eq!(
            super::super::finish(&checker.queries, &checker.query_budgets)
                .unwrap_err()
                .code,
            "B001"
        );
    }

    #[test]
    pub(crate) fn query_scopes_distinguish_nested_restart_targets() {
        let checker = check(
            "p:@\"proof\";flag:=false;'outer{a:p.can_copy<uint8>();'inner{b:p.can_copy<uint16>();|flag|'outer.restart();|flag|'inner.restart()};c:p.can_copy<uint32>()}",
        );
        assert_eq!(checker.restart_queries[&0], BTreeSet::from([0, 1, 2]));
        assert_eq!(checker.restart_queries[&1], BTreeSet::from([1]));
        assert_ne!(
            checker.restart_inputs[&0].target,
            checker.restart_inputs[&1].target
        );
    }

    #[test]
    pub(crate) fn query_copies_preserve_original_scope_and_function_owner() {
        let checker = check(
            "p:@\"proof\";flag:=false;original:p.can_copy<uint8>();'outer{copy:original;f<int32>:(){q:p.can_copy<uint16>();->1};local:p.can_copy<uint32>();|flag|'outer.restart()}",
        );
        assert_eq!(checker.queries.len(), 3);
        assert_eq!(checker.restart_queries[&0], BTreeSet::from([2]));
        assert_ne!(checker.queries[1].owner, checker.restart_inputs[&0].owner);
        assert!(
            !checker.queries[0]
                .scopes
                .contains(&checker.restart_inputs[&0].target)
        );
    }

    #[test]
    pub(crate) fn query_scope_recognition_and_errors_do_not_allocate_metadata() {
        let mut checker = Checker::new();
        let setup = crate::parser::parse("p:@\"proof\"").unwrap();
        checker.stmt(&setup.stmts[0]).unwrap();
        let block = crate::parser::parse("p.can_copy<uint8>()").unwrap();
        let crate::ast::StmtKind::Expr(expr) = &block.stmts[0].kind else {
            panic!()
        };
        assert!(checker.pending_form(expr).unwrap().is_some());
        assert!(checker.queries.is_empty());
        assert!(checker.restart_queries.is_empty());
        let block = crate::parser::parse("bad:p.can_copy<uint8>(1)").unwrap();
        assert_eq!(checker.stmt(&block.stmts[0]).unwrap_err().code, "E212");
        assert!(checker.queries.is_empty());
        assert!(checker.restart_queries.is_empty());
    }

    #[test]
    pub(crate) fn query_scope_and_association_work_are_bounded() {
        let mut checker = Checker::new();
        let block = crate::parser::parse("").unwrap();
        for _ in 0..MAX_QUERY_SCOPES {
            checker.block_start(&block, None, None, false).unwrap();
        }
        assert_eq!(
            checker.query_scopes(block.span).unwrap().len(),
            MAX_QUERY_SCOPES
        );
        checker.block_start(&block, None, None, false).unwrap();
        assert_eq!(checker.query_scopes(block.span).unwrap_err().code, "B001");
        let mut checker = check("p:@\"proof\";q:p.can_copy<uint8>()");
        assert!(!checker.flow.spend(usize::MAX));
        assert!(checker.track_restart_input(0, 0, Span::new(0, 1)).is_err());
        assert!(checker.restart_inputs.is_empty());
        assert!(checker.restart_queries.is_empty());
    }
}
