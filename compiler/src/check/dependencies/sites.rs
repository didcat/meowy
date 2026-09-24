use crate::{ast::Span, check::Checker, check::Result, diagnostic::Diagnostic, hir};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Site {
    pub(crate) point: Option<hir::PointId>,
    pub(crate) owner: usize,
    pub(crate) block: Option<hir::BlockId>,
    pub(crate) parent: Option<hir::StatementId>,
    pub(crate) span: Span,
    pub(crate) complete: bool,
}

impl Checker {
    pub(crate) fn checked_site(&mut self, span: Span) -> Result<Option<hir::StatementId>> {
        if !self
            .flow
            .spend(self.sites.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(Diagnostic::unsupported(
                "proof checked-site budget exhausted",
                span,
            ));
        }
        Ok(self.site.filter(|id| self.sites[id].owner == self.owner))
    }

    pub(crate) fn track_site(&mut self, id: hir::StatementId, span: Span) -> Result<()> {
        if !self.flow.spend(
            self.frames.len() + self.sites.len().checked_ilog2().unwrap_or(0) as usize * 2 + 2,
        ) {
            return Err(Diagnostic::unsupported(
                "proof checked-site budget exhausted",
                span,
            ));
        }
        let parent = self.site.filter(|id| self.sites[id].owner == self.owner);
        let block = self
            .frames
            .iter()
            .rev()
            .find(|frame| frame.owner == self.owner)
            .map(|frame| frame.id);
        self.sites.insert(
            id,
            Site {
                point: None,
                owner: self.owner,
                block,
                parent,
                span,
                complete: false,
            },
        );
        Ok(())
    }
}

#[cfg(test)]
mod uses {
    use super::*;

    pub(crate) fn check(source: &str) -> Checker {
        let mut checker = Checker::new();
        let block = crate::parser::parse(source).unwrap();
        checker.block(&block, None, None).unwrap();
        checker
    }

    pub(crate) fn seed() -> Checker {
        let mut checker = Checker::new();
        let block = crate::parser::parse("p:@\"proof\"").unwrap();
        checker.stmt(&block.stmts[0]).unwrap();
        checker
    }

    pub(crate) fn call(source: &str) -> crate::ast::Expr {
        let stmt = crate::parser::parse(source).unwrap().stmts.remove(0);
        let crate::ast::StmtKind::Bind { value, .. } = stmt.kind else {
            panic!()
        };
        value
    }

    #[test]
    pub(crate) fn erased_uses_retain_checked_statements_across_restarts_and_nested_blocks() {
        let source = "p:@\"proof\";n:3;flag:=false;'loop{<Before>:{-><uint8[n]>};q:p.can_copy<({-><uint8[n]>})>();copy:q;|flag|{<Inside>:{-><uint8[n]>};other:p.can_copy<uint16>()};|flag|'loop.restart();<After>:{-><uint8[n]>}}";
        let checker = check(source);
        let mut names = Vec::new();
        for (block, inputs) in &checker.body_inputs {
            for input in inputs {
                let site = &checker.sites[&input.site.unwrap()];
                assert_eq!(site.block, Some(*block));
                assert!(site.complete);
                names.push(&source[site.span.start..site.span.end]);
            }
        }
        for name in ["<Before>", "<Inside>", "<After>", "q:"] {
            assert_eq!(
                names.iter().filter(|text| text.starts_with(name)).count(),
                1
            );
        }
        assert_eq!(checker.queries.len(), 2);
        let query = &checker.queries[0];
        let site = &checker.sites[&query.site.unwrap()];
        let input = checker
            .body_inputs
            .values()
            .flatten()
            .find(|input| input.site == query.site)
            .unwrap();
        assert_eq!(site.owner, query.owner);
        assert_eq!(
            input.root,
            checker.query_budgets[query.root].as_ref().unwrap().root
        );
        assert!(source[site.span.start..site.span.end].starts_with("q:"));
        assert_ne!(query.site, checker.queries[1].site);
        assert_eq!(checker.restart_queries[&0].len(), 2);
        assert_eq!(crate::compile(source).unwrap_err()[0].code, "B001");
    }

    #[test]
    pub(crate) fn erased_queries_keep_function_sites_and_original_copy_origins() {
        let source = "p:@\"proof\";n:3;q:p.can_copy<uint8>();f:(){own:p.can_copy<({-><uint8[n]>})>();copy:own};last:q";
        let checker = check(source);
        assert_eq!(checker.queries.len(), 2);
        let outer = &checker.queries[0];
        let inner = &checker.queries[1];
        let site = &checker.sites[&inner.site.unwrap()];
        assert_ne!(outer.owner, inner.owner);
        assert_eq!(site.owner, inner.owner);
        assert!(site.parent.is_none());
        let block = checker.functions[0].as_ref().unwrap().body.id;
        assert_eq!(site.block, Some(block));
        assert_eq!(checker.body_inputs[&block][0].site, inner.site);
        let site = &checker.sites[&outer.site.unwrap()];
        assert!(source[site.span.start..site.span.end].starts_with("q:"));
        let source = "p:@\"proof\";q:p.can_copy<uint8>();f:(){copy:q}";
        assert_eq!(crate::compile(source).unwrap_err()[0].code, "E223");
    }

    #[test]
    pub(crate) fn erased_site_capture_preserves_recognition_purity_and_failed_preparation() {
        let mut checker = seed();
        let count = checker.sites.len();
        for source in ["q:p.can_copy<uint8>()", "q:p.can_copy<uint8>(1)"] {
            let expr = call(source);
            let _ = checker.pending_form(&expr);
            assert_eq!(checker.sites.len(), count);
            assert!(checker.queries.is_empty());
            assert!(checker.body_inputs.is_empty());
            assert!(checker.query_budgets.is_empty());
        }
        let block = crate::parser::parse("q:p.can_copy<Missing>()").unwrap();
        assert!(checker.block(&block, None, None).is_err());
        assert!(checker.queries.is_empty());
        assert!(checker.query_budgets.is_empty());
        assert!(checker.site.is_none());
        assert!(!checker.sites.values().last().unwrap().complete);
    }

    #[test]
    pub(crate) fn erased_uses_exclude_skipped_reads_and_fixed_query_signatures() {
        let checker = check(
            "p:@\"proof\";n:3;flag:true;q:p.can_copy<uint8>();<A>:q.always<>;<B>:{|false|x:n;-><uint8>};<C>:{b:false&&flag;-><uint8>}",
        );
        assert!(checker.body_inputs.is_empty());
        assert_eq!(checker.queries.len(), 1);
        assert!(checker.queries[0].site.is_some());
        assert!(checker.sites.values().all(|site| site.complete));
    }

    #[test]
    pub(crate) fn erased_uses_do_not_invent_sites_outside_statement_checking() {
        let mut checker = seed();
        let count = checker.sites.len();
        let expr = call("q:p.can_copy<uint8>()");
        let id = checker.pending_query(&expr).unwrap().unwrap();
        assert!(checker.queries[id].site.is_none());
        assert_eq!(checker.sites.len(), count);
        checker.site = Some(0);
        checker.owner += 1;
        assert!(checker.checked_site(expr.span).unwrap().is_none());
        assert!(!checker.flow.spend(usize::MAX));
        assert_eq!(checker.checked_site(expr.span).unwrap_err().code, "B001");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(crate) fn checked_statement_roots_cover_erased_values_and_preserve_failed_sites() {
        let mut checker = Checker::new();
        for (source, erased) in [("<T>:<uint8>", true), ("1", false)] {
            let stmt = crate::parser::parse(source).unwrap().stmts.remove(0);
            let (id, stmts) = checker.checked_stmt(&stmt).unwrap();
            assert_eq!(stmts.is_empty(), erased);
            let point = &checker.points[id];
            let site = &checker.sites[&point.site.unwrap()];
            assert_eq!(site.point, Some(id));
            assert_eq!(point.kind, super::super::PointKind::Stmt);
            assert_eq!(point.owner, site.owner);
            assert_eq!(point.block, site.block);
            assert!(point.complete && site.complete);
            assert!(
                !stmts
                    .iter()
                    .any(|stmt| matches!(stmt, hir::Stmt::Statement { .. }))
            );
        }
        let stmt = crate::parser::parse("bad<boolean>:1")
            .unwrap()
            .stmts
            .remove(0);
        assert_eq!(checker.checked_stmt(&stmt).unwrap_err().code, "E207");
        let site = checker.sites.values().last().unwrap();
        assert!(!site.complete);
        assert!(site.point.is_none());
        assert!(checker.statement.is_empty());
        assert!(checker.point.is_none());
    }

    #[test]
    pub(crate) fn checked_sites_distinguish_identical_spans_and_erased_statements() {
        let mut checker = Checker::new();
        let stmt = crate::parser::parse("1").unwrap().stmts.remove(0);
        checker.stmt(&stmt).unwrap();
        checker.stmt(&stmt).unwrap();
        assert_eq!(checker.sites.len(), 2);
        assert_eq!(checker.sites[&0].span, checker.sites[&1].span);
        assert!(checker.sites.values().all(|site| site.complete));
        assert!(checker.sites.values().all(|site| site.parent.is_none()));
        assert!(checker.sites.values().all(|site| site.block.is_none()));
        let stmt = crate::parser::parse("<T>:<uint8>").unwrap().stmts.remove(0);
        assert!(checker.stmt(&stmt).unwrap().is_empty());
        assert!(checker.sites[&2].complete);
        assert!(checker.site.is_none());
    }

    #[test]
    pub(crate) fn checked_sites_retain_block_and_same_function_containment() {
        let source = "'outer{a:1;'inner{b:2};f:(){c:3}}";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        let block = crate::parser::parse(source).unwrap();
        let root = checker.block(&block, None, None).unwrap();
        let sites = checker.sites.values().collect::<Vec<_>>();
        assert_eq!(sites.len(), 6);
        assert_eq!(sites[0].block, Some(root.id));
        assert_eq!(sites[1].parent, Some(0));
        assert_eq!(sites[2].parent, Some(0));
        assert_eq!(sites[3].parent, Some(2));
        assert_eq!(sites[1].block, sites[2].block);
        assert_ne!(sites[2].block, sites[3].block);
        let function = checker.functions[0].as_ref().unwrap();
        let local = sites.last().unwrap();
        assert_eq!(local.block, Some(function.body.id));
        assert_ne!(local.owner, sites[0].owner);
        assert!(local.parent.is_none());
        assert!(sites.iter().all(|site| site.complete));
    }

    #[test]
    pub(crate) fn checked_sites_restore_context_and_retain_incomplete_failures() {
        let mut checker = Checker::new();
        let block = crate::parser::parse("{a:1;b<boolean>:2}").unwrap();
        let error = checker.block(&block, None, None).unwrap_err();
        assert_eq!(error.code, "E207");
        assert!(checker.site.is_none());
        assert!(checker.statement.is_empty());
        assert!(!checker.sites[&0].complete);
        assert!(checker.sites[&1].complete);
        assert!(!checker.sites[&2].complete);
        let stmt = crate::parser::parse("next:3").unwrap().stmts.remove(0);
        checker.stmt(&stmt).unwrap();
        assert!(checker.sites[&3].complete);
        assert!(checker.sites[&3].parent.is_none());
    }

    #[test]
    pub(crate) fn checked_sites_preserve_statement_capacity_and_flow_bounds() {
        for capacity in [false, true] {
            let mut checker = Checker::new();
            if capacity {
                checker.statements = 65_536;
            } else {
                assert!(!checker.flow.spend(usize::MAX));
            }
            let stmt = crate::parser::parse("x:1").unwrap().stmts.remove(0);
            assert_eq!(checker.stmt(&stmt).unwrap_err().code, "B001");
            assert!(checker.sites.is_empty());
            assert!(checker.site.is_none());
            assert!(checker.statement.is_empty());
        }
    }
}
