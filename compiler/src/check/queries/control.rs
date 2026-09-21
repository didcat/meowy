use super::*;

pub(crate) fn check(source: &str) -> Checker {
    let mut checker = Checker::new();
    checker.derived.insert(0);
    let block = crate::parser::parse(source).unwrap();
    checker.block(&block, None, None).unwrap();
    checker
}

#[test]
pub(crate) fn controlled_query_availability_keeps_original_call_and_nested_control() {
    for condition in ["flag", "!flag", "false&&flag"] {
        let source = format!(
            "flag:false;p:@\"proof\";|{condition}|{{|true|{{r:p.can_copy<uint32>();copy:r}}}}"
        );
        let checker = check(&source);
        assert_eq!(checker.queries.len(), 1);
        assert!(checker.queries[0].control);
        assert!(!checker.control);
        let error = finish(&checker.queries, &checker.query_budgets).unwrap_err();
        assert_eq!(error.code, "E225");
        assert_eq!(
            &source[error.span.start..error.span.end],
            "p.can_copy<uint32>()"
        );
    }
}

#[test]
pub(crate) fn controlled_copies_do_not_create_observations_or_taint_later_queries() {
    let source = "flag:false;p:@\"proof\";r:p.can_copy<uint32>();|flag|{copy:r;<Flag>:r.always<>};later:p.can_copy<uint8>()";
    let checker = check(source);
    assert_eq!(checker.queries.len(), 2);
    assert!(checker.queries.iter().all(|query| !query.control));
    assert_eq!(
        finish(&checker.queries, &checker.query_budgets)
            .unwrap_err()
            .code,
        "B001"
    );
}

#[test]
pub(crate) fn later_controlled_query_is_not_hidden_by_an_unevaluated_earlier_query() {
    let source = "flag:false;p:@\"proof\";first:p.can_copy<uint8>();|flag|r:p.can_copy<uint32>();last:p.can_copy<uint16>()";
    let mut checker = check(source);
    assert_eq!(
        checker
            .queries
            .iter()
            .map(|query| query.control)
            .collect::<Vec<_>>(),
        [false, true, false]
    );
    let error = finish(&checker.queries, &checker.query_budgets).unwrap_err();
    assert_eq!(error.code, "E225");
    assert_eq!(error.span, checker.queries[1].span);
    let root = checker.queries[0].root;
    let span = checker.queries[0].span;
    checker.query_budgets[root].as_mut().unwrap().failure =
        Some(Checker::error("E220", "retained budget failure", span));
    let error = finish(&checker.queries, &checker.query_budgets).unwrap_err();
    assert_eq!(error.code, "E220");
    assert_eq!(error.span, span);
}

#[test]
pub(crate) fn controlled_query_metadata_preserves_ordinary_errors() {
    for (tail, code) in [
        ("bad<int32>:false", "E207"),
        ("x:=7;r:&x;x=8;copy:*r", "E302"),
    ] {
        let source = format!("flag:false;p:@\"proof\";|flag|r:p.can_copy<uint32>();{tail}");
        let mut checker = Checker::new();
        checker.derived.insert(0);
        let block = crate::parser::parse(&source).unwrap();
        let result = checker.block(&block, None, None);
        let error = match result {
            Err(error) => error,
            Ok(body) => {
                let program = crate::hir::Program {
                    body,
                    functions: Vec::new(),
                    locals: checker.locals.clone(),
                };
                checker.proofs.conditions = checker.guards.clone();
                checker.proofs.tags = checker.tags.clone();
                let facts =
                    crate::borrow::check(&program, &mut checker.flow, &checker.proofs).unwrap();
                crate::loans::check(&program, &facts, &checker.proofs, &mut checker.flow)
                    .unwrap_err()
                    .remove(0)
            }
        };
        assert_eq!(error.code, code);
        assert!(checker.queries[0].control);
    }
}
