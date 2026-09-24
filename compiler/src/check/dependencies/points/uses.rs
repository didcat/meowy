use super::*;

pub(crate) fn check(source: &str) -> Checker {
    let mut checker = Checker::new();
    let block = crate::parser::parse(source).unwrap();
    checker.block(&block, None, None).unwrap();
    checker
}

#[test]
pub(crate) fn required_points_distinguish_repeated_reads_inside_one_query() {
    let source = "p:@\"proof\";n:3;q:p.can_copy<({-><uint8[n+n]>})>();copy:q";
    let checker = check(source);
    assert_eq!(checker.queries.len(), 1);
    let query = &checker.queries[0];
    let point = &checker.points[query.point];
    assert_eq!(point.kind, Kind::Query);
    assert_eq!(point.site, query.site);
    assert!(point.complete);
    let reads = checker.body_inputs.values().flatten().collect::<Vec<_>>();
    assert_eq!(reads.len(), 2);
    assert_ne!(reads[0].point, reads[1].point);
    assert_eq!(reads[0].site, reads[1].site);
    for read in reads {
        let point = &checker.points[read.point];
        assert_eq!(point.kind, Kind::Read);
        assert_eq!(point.parent, Some(query.point));
        assert_eq!(point.site, query.site);
        assert_eq!(point.span, read.span);
        assert!(point.complete);
        assert_eq!(
            read.root,
            checker.query_budgets[query.root].as_ref().unwrap().root
        );
    }
    assert_eq!(
        checker
            .points
            .iter()
            .filter(|point| point.kind == Kind::Query)
            .count(),
        1
    );
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "B001");
}

#[test]
pub(crate) fn required_points_preserve_original_failures_and_incomplete_queries() {
    let source = "p:@\"proof\";n:3;q:p.can_copy<({-><uint8[n/0]>})>()";
    let mut checker = Checker::new();
    let block = crate::parser::parse(source).unwrap();
    assert_eq!(checker.block(&block, None, None).unwrap_err().code, "E107");
    assert!(checker.queries.is_empty());
    assert!(checker.point.is_none());
    assert!(checker.type_work.is_none());
    let read = checker.body_inputs.values().flatten().next().unwrap();
    let read = &checker.points[read.point];
    assert!(read.complete);
    let query = &checker.points[read.parent.unwrap()];
    assert_eq!(query.kind, Kind::Query);
    assert!(!query.complete);
}

#[test]
pub(crate) fn required_points_exclude_skipped_reads_and_fixed_signatures() {
    let checker =
        check("p:@\"proof\";n:3;q:p.can_copy<uint8>();<F>:q.always<>;<T>:{|false|x:n;-><uint8>}");
    assert!(checker.body_inputs.is_empty());
    assert!(!checker.points.iter().any(|point| point.kind == Kind::Read));
    assert_eq!(
        checker
            .points
            .iter()
            .filter(|point| point.kind == Kind::Query)
            .count(),
        1
    );
    let mut checker = Checker::new();
    let block = crate::parser::parse("p:@\"proof\";q:p.can_copy<uint8>()").unwrap();
    checker.stmt(&block.stmts[0]).unwrap();
    let crate::ast::StmtKind::Bind { value, .. } = &block.stmts[1].kind else {
        panic!()
    };
    let count = checker.points.len();
    assert!(checker.pending_form(value).unwrap().is_some());
    assert_eq!(checker.points.len(), count);
}

#[test]
pub(crate) fn required_points_keep_nested_functions_separate_from_expression_parents() {
    let checker = check("p:@\"proof\";n:3;x:{f:(){q:p.can_copy<({-><uint8[n]>})>()};->0}");
    let query = &checker.queries[0];
    let point = &checker.points[query.point];
    assert_eq!(point.owner, query.owner);
    let stmt = &checker.points[point.parent.unwrap()];
    assert_eq!(stmt.kind, Kind::Stmt);
    assert_eq!(stmt.owner, query.owner);
    assert!(stmt.parent.is_none());
    assert_eq!(
        point.block,
        Some(checker.functions[0].as_ref().unwrap().body.id)
    );
    let read = checker.body_inputs.values().flatten().next().unwrap();
    assert_eq!(checker.points[read.point].parent, Some(query.point));
}
