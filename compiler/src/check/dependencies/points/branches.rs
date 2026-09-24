use super::{uses::check, *};

pub(crate) fn children(checker: &Checker, id: usize) -> Vec<Kind> {
    checker
        .points
        .iter()
        .filter(|point| point.parent == Some(id))
        .map(|point| point.kind)
        .collect()
}

#[test]
pub(crate) fn branch_points_attach_inline_queries_and_reads_to_matcher_arms() {
    let checker = check(
        "p:@\"proof\";n:3;flag:=false;|flag|q:p.can_copy<({-><uint8[n]>})>();|!flag|<T>:{-><uint8[n]>};after:p.can_copy<uint16>()",
    );
    let branches = checker
        .points
        .iter()
        .enumerate()
        .filter(|(_, point)| point.kind == Kind::Match)
        .map(|(id, _)| id)
        .collect::<Vec<_>>();
    assert_eq!(branches.len(), 2);
    for id in &branches {
        assert_eq!(
            children(&checker, *id),
            [Kind::Condition, Kind::Then, Kind::Else]
        );
        assert!(checker.points[*id].complete);
    }
    let query = &checker.points[checker.queries[0].point];
    let body = &checker.points[query.parent.unwrap()];
    assert_eq!(body.kind, Kind::Stmt);
    let arm = &checker.points[body.parent.unwrap()];
    assert_eq!(arm.kind, Kind::Then);
    assert_eq!(arm.parent, Some(branches[0]));
    let reads = checker.body_inputs.values().flatten().collect::<Vec<_>>();
    assert_eq!(reads.len(), 2);
    let body = checker.points[reads[1].point].parent.unwrap();
    assert_eq!(checker.points[body].kind, Kind::Stmt);
    let arm = checker.points[body].parent.unwrap();
    assert_eq!(checker.points[arm].kind, Kind::Then);
    assert_eq!(checker.points[arm].parent, Some(branches[1]));
    assert!(checker.points[checker.queries[1].point].parent.is_none());
}

#[test]
pub(crate) fn branch_points_keep_both_short_circuit_alternatives_and_erased_rhs_uses() {
    for (op, left, branch, arm) in [
        ("&&", "false", Kind::And, Kind::Then),
        ("||", "true", Kind::Or, Kind::Else),
    ] {
        let source = format!(
            "p:@\"proof\";n:3;value:{left}{op}{{q:p.can_copy<({{-><uint8[n]>}})>();->true}}"
        );
        let checker = check(&source);
        let id = checker
            .points
            .iter()
            .position(|point| point.kind == branch)
            .unwrap();
        let other = if arm == Kind::Then {
            Kind::Else
        } else {
            Kind::Then
        };
        assert_eq!(children(&checker, id), [Kind::Condition, arm, other]);
        let query = &checker.points[checker.queries[0].point];
        let expr = &checker.points[query.parent.unwrap()];
        let region = &checker.points[expr.parent.unwrap()];
        assert_eq!(expr.kind, Kind::Expr);
        assert_eq!(region.kind, arm);
        assert_eq!(region.parent, Some(id));
        assert!(checker.points.iter().all(|point| point.complete));
        assert_eq!(crate::compile(&source).unwrap_err()[0].code, "B001");
    }
}

#[test]
pub(crate) fn branch_points_restore_context_after_errors_and_nested_function_checks() {
    for (source, code) in [
        ("|1|2", "E215"),
        ("|false|x<boolean>:1", "E207"),
        ("false&&1", "E222"),
    ] {
        let mut checker = Checker::new();
        let block = crate::parser::parse(source).unwrap();
        assert_eq!(checker.block(&block, None, None).unwrap_err().code, code);
        assert!(checker.point.is_none());
        assert!(!checker.points[0].complete);
        assert!(!checker.control);
    }
    let checker =
        check("p:@\"proof\";|false|{f:(){q:p.can_copy<uint8>()};outer:p.can_copy<uint16>()}");
    assert!(checker.points[checker.queries[0].point].parent.is_none());
    assert!(checker.points[checker.queries[1].point].parent.is_some());
    for point in &checker.points {
        if let Some(parent) = point.parent {
            assert_eq!(point.owner, checker.points[parent].owner);
        }
    }
}

#[test]
pub(crate) fn branch_points_preserve_independent_nested_short_circuit_conditions() {
    let checker = check("a:=false;b:=true;c:=false;x:a&&(b||c)");
    let outer = checker
        .points
        .iter()
        .position(|point| point.kind == Kind::And)
        .unwrap();
    let inner = checker
        .points
        .iter()
        .position(|point| point.kind == Kind::Or)
        .unwrap();
    assert_eq!(
        children(&checker, outer),
        [Kind::Condition, Kind::Then, Kind::Else]
    );
    assert_eq!(
        children(&checker, inner),
        [Kind::Condition, Kind::Else, Kind::Then]
    );
    let group = checker.points[inner].parent.unwrap();
    let arm = checker.points[group].parent.unwrap();
    assert_eq!(checker.points[arm].kind, Kind::Then);
    assert_eq!(checker.points[arm].parent, Some(outer));
}
