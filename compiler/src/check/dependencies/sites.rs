use crate::{ast::Span, check::Checker, check::Result, diagnostic::Diagnostic, hir};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Site {
    pub(crate) owner: usize,
    pub(crate) block: Option<hir::BlockId>,
    pub(crate) parent: Option<hir::StatementId>,
    pub(crate) span: Span,
    pub(crate) complete: bool,
}

impl Checker {
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
mod tests {
    use super::*;

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
