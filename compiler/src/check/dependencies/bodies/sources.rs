use super::{tests::check, *};

#[test]
pub(crate) fn matcher_sources_link_body_facts_to_erased_query_and_read_regions() {
    let source = "p:@\"proof\";n:3;flag:=false;|flag|q:p.can_copy<({-><uint8[n]>})>()";
    let (checker, block) = check(source);
    let body = &checker.bodies[&block.id];
    assert_eq!(body.facts.len(), body.sources.len());
    let index = body
        .facts
        .iter()
        .position(|(fact, _)| *fact == Fact::Branch)
        .unwrap();
    let point = body.sources[index].unwrap();
    let query = &checker.queries[0];
    let stmt = checker.points[query.point].parent.unwrap();
    let arm = checker.points[stmt].parent.unwrap();
    assert_eq!(checker.points[arm].parent, Some(point));
    let read = &checker.body_inputs[&block.id][0];
    assert_eq!(checker.points[read.point].parent, Some(query.point));
    assert_eq!(checker.points[point].kind, super::super::PointKind::Match);
    assert!(
        body.facts
            .iter()
            .zip(&body.sources)
            .all(|((fact, _), source)| { source.is_none() || *fact == Fact::Branch })
    );
}

#[test]
pub(crate) fn matcher_sources_keep_nested_and_function_blocks_distinct() {
    let (checker, _) = check("flag:=false;|flag|{|true|0};f:(){|false|1}");
    let mut count = 0;
    for (id, body) in &checker.bodies {
        for source in body.sources.iter().flatten() {
            let point = &checker.points[*source];
            assert_eq!(point.owner, body.owner);
            assert_eq!(point.block, Some(*id));
            count += 1;
        }
    }
    assert_eq!(count, 3);
}

#[test]
pub(crate) fn matcher_sources_reject_invalid_provenance_without_replacing_body_facts() {
    for case in 0..5 {
        let (mut checker, mut block) = check("|true|1");
        let hir::Stmt::If { point, .. } = &mut block.stmts[0] else {
            panic!()
        };
        let id = point.unwrap();
        match case {
            0 => *point = Some(usize::MAX),
            1 => checker.points[id].kind = super::super::PointKind::Read,
            2 => checker.points[id].owner += 1,
            3 => checker.points[id].block = None,
            4 => checker.points[id].complete = false,
            _ => unreachable!(),
        }
        let count = checker.body_facts;
        let error = checker
            .track_body(&block, Span { start: 0, end: 7 })
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("source identity mismatch"));
        assert_eq!(checker.body_facts, count);
        assert_eq!(checker.bodies[&block.id].sources, [Some(id)]);
    }
}

#[test]
pub(crate) fn matcher_sources_keep_synthetic_provenance_explicitly_unknown() {
    let (_, mut block) = check("|true|1");
    let hir::Stmt::If { point, .. } = &mut block.stmts[0] else {
        panic!()
    };
    *point = None;
    let mut checker = Checker::new();
    checker
        .track_body(&block, Span { start: 0, end: 7 })
        .unwrap();
    assert!(checker.points.is_empty());
    assert_eq!(checker.bodies[&block.id].sources, [None]);
}

#[test]
pub(crate) fn logic_sources_keep_mixed_nested_matchers_and_function_bodies_exact() {
    let source = "flag:=false;|flag&&(true||false)|{a:true||false};f<boolean>:(){->true&&false}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let mut count = 0;
    for (id, body) in &checker.bodies {
        for ((fact, _), source) in body.facts.iter().zip(&body.sources) {
            let kind = match fact {
                Fact::And => super::super::PointKind::And,
                Fact::Or => super::super::PointKind::Or,
                Fact::Branch => super::super::PointKind::Match,
                _ => {
                    assert!(source.is_none());
                    continue;
                }
            };
            let point = &checker.points[source.unwrap()];
            assert_eq!(point.kind, kind);
            assert_eq!(point.owner, body.owner);
            assert_eq!(point.block, Some(*id));
            assert!(point.complete);
            count += 1;
        }
    }
    assert_eq!(count, 5);
}

#[test]
pub(crate) fn logic_sources_link_skipped_rhs_queries_and_reads_across_nested_blocks() {
    for (left, op, fact) in [("false", "&&", Fact::And), ("true", "||", Fact::Or)] {
        let source =
            format!("p:@\"proof\";n:3;x:{left}{op}{{q:p.can_copy<({{-><uint8[n]>}})>();->true}}");
        let (checker, block) = check(&source);
        let body = &checker.bodies[&block.id];
        let index = body
            .facts
            .iter()
            .position(|(found, _)| *found == fact)
            .unwrap();
        let point = body.sources[index].unwrap();
        let query = &checker.queries[0];
        let stmt = checker.points[query.point].parent.unwrap();
        let expr = checker.points[stmt].parent.unwrap();
        let region = checker.points[expr].parent.unwrap();
        assert_eq!(checker.points[region].parent, Some(point));
        let read = checker.body_inputs.values().flatten().next().unwrap();
        assert_eq!(checker.points[read.point].parent, Some(query.point));
        assert_ne!(checker.points[query.point].block, Some(block.id));
        assert_eq!(crate::compile(&source).unwrap_err()[0].code, "B001");
    }
}

#[test]
pub(crate) fn logic_sources_reject_mismatched_operators_and_keep_unknown_origins() {
    let (mut checker, mut block) = check("false&&true");
    let hir::Stmt::Expr(expr) = &mut block.stmts[0] else {
        panic!()
    };
    let hir::ExprKind::Binary { op, .. } = &mut expr.kind else {
        panic!()
    };
    *op = "||".into();
    let span = Span { start: 0, end: 11 };
    let error = checker.track_body(&block, span).unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("source identity mismatch"));
    let hir::Stmt::Expr(expr) = &mut block.stmts[0] else {
        panic!()
    };
    let hir::ExprKind::Binary { point, .. } = &mut expr.kind else {
        panic!()
    };
    *point = None;
    let mut checker = Checker::new();
    checker.track_body(&block, span).unwrap();
    assert_eq!(checker.bodies[&block.id].sources, [None]);
}
