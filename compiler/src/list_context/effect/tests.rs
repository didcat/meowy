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

pub(crate) fn candidates() -> [Type; 2] {
    [8, 32].map(|bits| Type::List {
        element: Box::new(Type::Int {
            bits,
            signed: bits == 32,
        }),
        capacity: 1,
    })
}

#[test]
pub(crate) fn effect_roots_capture_original_grouping_owner_and_once_only_bindings() {
    let mut checker = Checker::new();
    checker.owner = 7;
    let value = expression("(({x:300;->x}))");
    let types = candidates();
    let mut choices = types.iter().collect();
    let scopes = checker.scopes.len();
    let (point, checked) = checker
        .list_effect_block(&value, &mut choices, false)
        .unwrap()
        .unwrap();
    assert_eq!(checker.points[point].span, value.span);
    assert_eq!(checker.points[point].owner, 7);
    assert!(checker.points[point].complete);
    assert_eq!(checked.span, value.span);
    assert_eq!(choices, [&types[1]]);
    assert_eq!(checker.operations.len(), 1);
    assert!(checker.operations.values().all(|op| op.owner == 7));
    assert_eq!(checker.scopes.len(), scopes);
    assert!(checker.frames.is_empty());
    assert!(checker.point.is_none());
    let hir::ExprKind::Block(body) = checked.kind else {
        panic!()
    };
    assert_eq!(checker.bodies[&body.id].parent, Some(point));
}

#[test]
pub(crate) fn effect_root_failures_restore_scopes_owner_and_active_context() {
    for (source, code) in [
        ("{x:missing;->x}", "E201"),
        ("{f:(){missing};->300}", "E201"),
        ("{x:1;->1}", "E207"),
    ] {
        let mut checker = Checker::new();
        let scopes = checker.scopes.len();
        let reach = checker.reach;
        let types = candidates();
        let mut choices = types.iter().collect();
        let error = checker
            .list_effect_block(&expression(source), &mut choices, false)
            .unwrap_err();
        assert_eq!(error.code, code);
        assert_eq!(choices.len(), 2);
        assert_eq!(checker.scopes.len(), scopes);
        assert!(checker.frames.is_empty());
        assert_eq!(checker.owner, 0);
        assert_eq!(checker.reach, reach);
        assert!(checker.point.is_none());
        assert!(!checker.points[0].complete);
    }
    let mut checker = Checker::new();
    let types = candidates();
    let mut choices = types.iter().collect();
    assert!(
        checker
            .list_effect_block(&expression("{->1}"), &mut choices, false)
            .unwrap()
            .is_none()
    );
    assert!(checker.points.is_empty());
}
