use super::*;

pub(crate) fn expression(source: &str) -> ast::Expr {
    let stmt = crate::parser::parse(source).unwrap().stmts.remove(0);
    let StmtKind::Expr(value) = stmt.kind else {
        panic!()
    };
    value
}

#[test]
pub(crate) fn effect_form_recognition_keeps_grouped_spans_without_checking_prefixes() {
    let mut checker = Checker::new();
    let value = expression("(({x:missing;->x}))");
    let (form, start) = checker.list_effect_form(&value).unwrap().unwrap();
    assert_eq!(start, 1);
    assert!(form.span.start > value.span.start);
    assert!(checker.points.is_empty());
    assert!(checker.locals.is_empty());
    assert!(checker.frames.is_empty());
    assert!(checker.operations.is_empty());
    assert!(checker.site.is_none());
}

#[test]
pub(crate) fn unsupported_effect_forms_leave_metadata_and_scopes_untouched() {
    for source in ["1", "{->1}", "'named{x:1;->x}", "{x:1}", "{x:1;->x;y:2}"] {
        let mut checker = Checker::new();
        let depth = checker.scopes.len();
        assert!(
            checker
                .list_effect_form(&expression(source))
                .unwrap()
                .is_none()
        );
        assert_eq!(checker.scopes.len(), depth);
        assert!(checker.frames.is_empty());
        assert!(checker.points.is_empty());
        assert!(checker.locals.is_empty());
    }
}

#[test]
pub(crate) fn effect_form_scans_preserve_their_budget_before_allocating_roots() {
    let mut checker = Checker::new();
    checker.flow.spend(usize::MAX);
    let error = checker
        .list_effect_form(&expression("{x:1;->x}"))
        .unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("effectful list block budget"));
    assert!(checker.points.is_empty());
}
