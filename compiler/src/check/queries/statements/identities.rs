use crate::check::{
    Value,
    queries::{Prepared, accounting::checker},
};

#[test]
pub(crate) fn pending_statements_return_exact_ids_for_calls_copies_and_discards() {
    let ast = crate::parser::parse("first:p.can_copy<uint8>();second<p.Result>:p.can_copy<uint16>();copy:(first);(second);p.can_copy<uint32>();again:copy").unwrap();
    let mut checker = checker();
    for (stmt, (id, created)) in ast.stmts.iter().zip([
        (0, true),
        (1, true),
        (0, false),
        (1, false),
        (2, true),
        (0, false),
    ]) {
        assert_eq!(
            checker.pending_statement(stmt).unwrap(),
            Some(Prepared { id, created })
        );
    }
    for (name, expected) in [("first", 0), ("second", 1), ("copy", 0), ("again", 0)] {
        assert!(
            matches!(checker.value(name, ast.span).unwrap(), Value::Pending(id) if id == expected)
        );
    }
    assert_eq!(checker.queries.len(), 3);
    assert_eq!(checker.query_budgets.len(), 3);
    assert!(checker.locals.is_empty());
    assert!(checker.type_work.is_none());
    assert!(
        checker
            .queries
            .iter()
            .all(|query| checker.points[query.point].complete)
    );
}

#[test]
pub(crate) fn pending_statements_return_no_identity_before_annotations_and_names_succeed() {
    for (source, code) in [
        ("bad<p.Always>:p.can_copy<uint8>()", "E207"),
        ("bad<int32>:p.can_copy<uint8>()", "E223"),
        ("bad:=p.can_copy<uint8>()", "E223"),
        ("bad:p.can_copy<Missing>()", "E202"),
        ("bad:p.can_copy<uint8,uint16>()", "E212"),
        ("bad:p.can_copy<uint8>(1)", "E212"),
    ] {
        let mut checker = checker();
        let ast = crate::parser::parse(source).unwrap();
        assert_eq!(
            checker.pending_statement(&ast.stmts[0]).unwrap_err().code,
            code,
            "{source}"
        );
        assert!(checker.value("bad", ast.span).is_err());
        assert!(checker.type_work.is_none());
    }
    let mut checker = checker();
    let ast = crate::parser::parse("r:p.can_copy<uint8>();r:p.can_copy<uint16>()").unwrap();
    assert_eq!(
        checker
            .pending_statement(&ast.stmts[0])
            .unwrap()
            .unwrap()
            .id,
        0
    );
    assert_eq!(
        checker.pending_statement(&ast.stmts[1]).unwrap_err().code,
        "E203"
    );
    assert!(matches!(
        checker.value("r", ast.span).unwrap(),
        Value::Pending(0)
    ));
}

#[test]
pub(crate) fn pending_statements_preserve_origin_owners_and_open_root_identity() {
    let mut checker = checker();
    let ast = crate::parser::parse("r:p.can_copy<uint8>();copy:r;p.can_copy<uint16>()").unwrap();
    checker
        .construction_root(ast.span, |checker| {
            let first = checker.pending_statement(&ast.stmts[0])?.unwrap();
            let copy = checker.pending_statement(&ast.stmts[1])?.unwrap();
            checker.owner = 4;
            assert_eq!(
                checker.pending_statement(&ast.stmts[1]).unwrap_err().code,
                "E223"
            );
            let later = checker.pending_statement(&ast.stmts[2])?.unwrap();
            assert_eq!(
                first,
                Prepared {
                    id: 0,
                    created: true
                }
            );
            assert_eq!(
                copy,
                Prepared {
                    id: 0,
                    created: false
                }
            );
            assert_eq!(
                later,
                Prepared {
                    id: 1,
                    created: true
                }
            );
            assert_eq!(checker.queries[copy.id].owner, 0);
            assert_eq!(checker.queries[later.id].owner, 4);
            assert_eq!(
                checker.queries[copy.id].root,
                checker.queries[later.id].root
            );
            assert!(checker.query_budgets[0].is_none());
            Ok(())
        })
        .unwrap();
    assert!(checker.query_budgets[0].is_some());
    assert!(checker.type_work.is_none());
    assert_eq!(
        crate::check::queries::finish(&checker.queries, &checker.query_budgets)
            .unwrap_err()
            .code,
        "B001"
    );
}
