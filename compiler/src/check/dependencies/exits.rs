use super::Checker;
use crate::{ast, check::Result, diagnostic::Diagnostic, hir};

impl Checker {
    pub(crate) fn continuation_control(&self) -> bool {
        self.frames
            .iter()
            .any(|frame| frame.owner == self.owner && frame.continuation)
    }

    pub(crate) fn continuation_statement(&mut self, stmt: &ast::Stmt) -> Result<Vec<hir::Stmt>> {
        self.with_continuation(stmt.span, "statement", |checker| checker.stmt_body(stmt))
    }

    pub(crate) fn with_continuation<T>(
        &mut self,
        span: ast::Span,
        kind: &str,
        check: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        if !self.flow.spend(self.frames.len() + 1) {
            return Err(Diagnostic::unsupported(
                format!("proof continuation {kind} budget exhausted"),
                span,
            ));
        }
        let prior = self
            .frames
            .iter()
            .map(|frame| frame.continuation)
            .collect::<Vec<_>>();
        let control = self.control;
        self.control |= self.continuation_control();
        let result = check(self);
        self.control = control;
        if result.is_err() {
            for (index, frame) in self.frames.iter_mut().enumerate() {
                frame.continuation = prior.get(index).copied().unwrap_or(false);
            }
        }
        result
    }

    pub(crate) fn track_leave_control(&mut self, index: usize, span: ast::Span) -> Result<()> {
        if !self.control {
            return Ok(());
        }
        if !self.flow.spend(self.frames.len() - index) {
            return Err(Diagnostic::unsupported(
                "proof leave continuation budget exhausted",
                span,
            ));
        }
        for frame in &mut self.frames[index..] {
            frame.continuation = true;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::dependencies::{carriers::id, tests::statements};

    #[test]
    pub(crate) fn conditional_leave_marks_successors_until_target_join() {
        let mut checker = Checker::new();
        statements(&mut checker, "flag:false");
        checker.mark_derived(id(&checker, "flag"));
        statements(&mut checker, "'out{|flag|'out.leave();n:3;copy:n};plain:7");
        let plain = id(&checker, "plain");
        assert!(!checker.derived_local(plain));
        assert!(checker.derived_local(plain - 1));
        assert!(checker.derived_local(plain - 2));
        assert!(!checker.control);
        assert!(!checker.continuation_control());
    }

    #[test]
    pub(crate) fn conditional_outer_leave_marks_intervening_successors_and_writes() {
        let mut checker = Checker::new();
        statements(&mut checker, "flag:false;x:=0");
        checker.mark_derived(id(&checker, "flag"));
        statements(
            &mut checker,
            "'out{'inner{|flag|'out.leave();a:3};x=7;b:4};plain:8",
        );
        assert!(checker.derived_local(id(&checker, "x")));
        let plain = id(&checker, "plain");
        assert!(checker.derived_local(plain - 1));
        assert!(checker.derived_local(plain - 2));
        assert!(!checker.derived_local(plain));
    }

    #[test]
    pub(crate) fn conditional_inner_leave_does_not_mark_outer_successors() {
        let mut checker = Checker::new();
        statements(&mut checker, "flag:false");
        checker.mark_derived(id(&checker, "flag"));
        statements(
            &mut checker,
            "'out{'inner{|flag|'inner.leave();inside:3};outside:4};plain:8",
        );
        let plain = id(&checker, "plain");
        assert!(checker.derived_local(plain - 2));
        assert!(!checker.derived_local(plain - 1));
        assert!(!checker.derived_local(plain));
    }

    #[test]
    pub(crate) fn leave_successor_queries_reject_derived_availability() {
        let mut checker = Checker::new();
        statements(&mut checker, "flag:false;p:@\"proof\"");
        checker.mark_derived(id(&checker, "flag"));
        statements(
            &mut checker,
            "'out{|flag|'out.leave();q:p.can_copy<uint32>()};plain:p.can_copy<uint8>()",
        );
        assert!(checker.queries[0].control);
        assert!(!checker.queries[1].control);
        assert_eq!(
            crate::check::queries::finish(&checker.queries, &checker.query_budgets)
                .unwrap_err()
                .code,
            "E225"
        );
    }

    #[test]
    pub(crate) fn leave_successor_required_values_preserve_error_precedence() {
        for (tail, code) in [
            ("n:3;<T>:{-><uint8[n]>}", "E225"),
            ("bad<int32>:false", "E207"),
        ] {
            let mut checker = Checker::new();
            statements(&mut checker, "flag:false");
            checker.mark_derived(id(&checker, "flag"));
            let block =
                crate::parser::parse(&format!("'out{{|flag|'out.leave();{tail}}}")).unwrap();
            assert_eq!(checker.stmt(&block.stmts[0]).unwrap_err().code, code);
            assert!(!checker.control);
            assert!(!checker.continuation_control());
        }
    }

    #[test]
    pub(crate) fn ordinary_leave_does_not_create_proof_control() {
        let source = "flag:=false;'out{|flag|'out.leave();n:3;<T>:{-><uint8[n]>}};plain:8";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        assert!(checker.derived.is_empty());
        assert!(!checker.continuation_control());
    }

    #[test]
    pub(crate) fn leave_continuations_cover_sibling_arms_and_bound_work() {
        let mut checker = Checker::new();
        statements(&mut checker, "flag:false");
        checker.mark_derived(id(&checker, "flag"));
        let mut block = crate::parser::parse("'out{|flag|'out.leave();|true|n:3}").unwrap();
        let ast::StmtKind::Expr(value) = &mut block.stmts[0].kind else {
            panic!()
        };
        let ast::ExprKind::Block(inner) = &mut value.kind else {
            panic!()
        };
        let second = inner.stmts.remove(1);
        let ast::StmtKind::Match { arms: next } = second.kind else {
            panic!()
        };
        let ast::StmtKind::Match { arms } = &mut inner.stmts[0].kind else {
            panic!()
        };
        arms.extend(next);
        checker.stmt(&block.stmts[0]).unwrap();
        assert!(checker.derived_local(checker.locals.len() - 1));
        assert!(!checker.continuation_control());
        assert!(!checker.flow.spend(usize::MAX));
        let stmt = crate::parser::parse("plain:3").unwrap().stmts.remove(0);
        let error = checker.stmt_inner(&stmt).unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("continuation statement budget"));
        assert!(!checker.control);
    }
}
