use crate::ast::{ExprKind, Span, StmtKind};
use crate::check::{Checker, required::MAX_TYPES};

pub(crate) fn cost(source: &str) -> (usize, usize) {
    let block = crate::parser::parse(source).unwrap();
    let mut checker = Checker::new();
    checker
        .mode_root(Span::new(1000, 1100), true, |checker| {
            checker.block(&block, None, None)?;
            let work = checker.type_work.as_ref().unwrap();
            assert!(work.ordinary);
            Ok((work.logical.steps, work.logical.types))
        })
        .unwrap()
}

#[test]
pub(crate) fn ascription_roots_charge_targets_once_and_keep_runtime_values_separate() {
    for (source, expected) in [
        ("v:7~<int32>", (1, 1)),
        ("|false|v:7~<(<int32><int32>)>", (5, 4)),
        ("v:(1+2)~<(<int32><int32>)>", (5, 4)),
        ("v:((7))~<(<int32><int32>)>", (5, 4)),
        ("v:7;|v<int32><int32>|copy:v", (3, 3)),
        ("v:[7];copy:v~<int32[1+0]>", (5, 2)),
        ("v:7~<({-><int32>})>", (6, 3)),
    ] {
        assert_eq!(cost(source), expected, "{source}");
    }
}

#[test]
pub(crate) fn ascription_roots_preserve_ordinary_extent_gates_and_error_order() {
    crate::compile("v:[7]~<({-><int32[({->1})]>})>").unwrap();
    for (source, code, token) in [
        ("v:(1/0)~<Missing>", "E107", "1/0"),
        ("v:7~<uint8>", "E208", "7~<uint8>"),
        ("v:[7]~<int32[1/0]>", "E107", "1/0"),
        ("v:[7]~<int32[({->1})]>", "B001", "({->1})"),
    ] {
        let error = crate::compile(source).unwrap_err().remove(0);
        assert_eq!(error.code, code, "{source}");
        assert_eq!(&source[error.span.start..error.span.end], token);
    }
}

#[test]
pub(crate) fn ascription_roots_share_limits_and_leave_target_probes_uncharged() {
    let block = crate::parser::parse("source:{->a:7};v:source~<{a<int32>}>").unwrap();
    let StmtKind::Bind { value, .. } = &block.stmts[1].kind else {
        panic!()
    };
    let ExprKind::Ascribe { ty, .. } = &value.kind else {
        panic!()
    };
    let root = Span::new(100, 150);
    for remaining in [2, 3, 4] {
        let mut checker = Checker::new();
        checker.stmt(&block.stmts[0]).unwrap();
        let result = checker.mode_root(root, true, |checker| {
            checker.ty(ty)?;
            assert_eq!(checker.type_work.as_ref().unwrap().logical.types, 0);
            checker.type_work.as_mut().unwrap().logical.types = MAX_TYPES - remaining;
            let result = checker.stmt(&block.stmts[1]);
            assert!(checker.type_work.as_ref().unwrap().ordinary);
            result
        });
        assert_eq!(result.is_ok(), remaining >= 3);
        if let Err(error) = result {
            assert_eq!(error.code, "E220");
            assert_eq!(error.span, root);
        }
        assert!(checker.type_work.is_none());
    }
}

#[test]
pub(crate) fn type_identity_bindings_charge_literals_reads_and_queries_once() {
    for (source, expected) in [
        ("kind:<int32>", (2, 1)),
        ("kind:(((<int32><int32>)))", (4, 3)),
        ("kind:<int32>;copy:kind;again:((kind))", (6, 3)),
        ("c:@\"core\";kind:((c)).int32", (3, 1)),
        ("m:@\"memory\";kind:m.Allocator;copy:kind", (5, 2)),
        ("n:7;kind:(n/0)<>;copy:kind", (4, 2)),
        ("kind:<({-><int32>})>", (7, 3)),
        ("kind:<int32>!<null>", (6, 3)),
        ("|false|kind:<int32><int32>", (4, 3)),
    ] {
        assert_eq!(cost(source), expected, "{source}");
    }
    let program = crate::compile("kind:<int32>;copy:kind").unwrap();
    assert!(program.body.stmts.is_empty());
    assert!(program.locals.is_empty());
}

#[test]
pub(crate) fn type_identity_bindings_preserve_extent_modes_and_source_failures() {
    crate::compile("kind:<({-><int32[({->2})]>})>;v<(kind)>:[7]").unwrap();
    for (source, code) in [
        ("kind:<int32[({->2})]>", "B001"),
        ("kind:<int32[1/0]><Missing>", "E107"),
        ("kind:<Missing><int32[1/0]>", "E202"),
        ("kind<null>:<int32>", "B001"),
        ("kind:=<int32>", "B001"),
        ("kind:(7~<int32[1/0]>)<>", "B001"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn type_identity_bindings_share_limits_and_do_not_charge_literal_probes() {
    let block = crate::parser::parse("kind:<int32><int32>").unwrap();
    let StmtKind::Bind { value, .. } = &block.stmts[0].kind else {
        panic!()
    };
    let root = Span::new(100, 150);
    for remaining in [2, 3, 4] {
        let mut checker = Checker::new();
        let result = checker.mode_root(root, true, |checker| {
            checker.symbol(value)?;
            assert_eq!(checker.type_work.as_ref().unwrap().logical.types, 0);
            checker.type_work.as_mut().unwrap().logical.types = MAX_TYPES - remaining;
            checker.stmt(&block.stmts[0])
        });
        assert_eq!(result.is_ok(), remaining >= 3);
        if let Err(error) = result {
            assert_eq!(error.code, "E220");
            assert_eq!(error.span, root);
            assert!(!checker.scopes.last().unwrap().values.contains_key("kind"));
            checker.stmt(&block.stmts[0]).unwrap();
        }
        assert!(checker.type_work.is_none());
    }
}

#[test]
pub(crate) fn field_hints_do_not_evaluate_computed_type_bases() {
    for source in [
        "kind:(<({-><int32>})>.missing)<>",
        "kind:((7<>).missing)<>",
        "kind:(<({-><int32[1/0]>})>.missing)<>",
        "kind:((<({-><int32>})>.missing).again)<>",
    ] {
        let block = crate::parser::parse(source).unwrap();
        let StmtKind::Bind { value, .. } = &block.stmts[0].kind else {
            panic!()
        };
        let ExprKind::TypeQuery(operand) = &value.kind else {
            panic!()
        };
        let mut checker = Checker::new();
        checker
            .mode_root(value.span, true, |checker| {
                assert!(checker.hint(operand).is_none());
                let work = checker.type_work.as_ref().unwrap();
                assert_eq!((work.visits, work.nodes), (0, 0), "{source}");
                assert_eq!((work.logical.steps, work.logical.types), (0, 0));
                assert!(work.ordinary);
                Ok(())
            })
            .unwrap();
        assert!(checker.type_work.is_none());
    }
}

#[test]
pub(crate) fn field_hint_failures_do_not_poison_query_roots() {
    let source = "kind:(<({-><int32>})>.missing)<>";
    let block = crate::parser::parse(source).unwrap();
    let StmtKind::Bind { value, .. } = &block.stmts[0].kind else {
        panic!()
    };
    let root = Span::new(100, 150);
    let mut checker = Checker::new();
    let error = checker
        .mode_root(root, true, |checker| {
            checker.type_work.as_mut().unwrap().logical.types = MAX_TYPES;
            checker.type_value(value)
        })
        .unwrap_err();
    assert_eq!(error.code, "B001");
    assert_eq!(error.span, value.span);
    assert!(
        error
            .message
            .contains("type queries requiring expression evaluation")
    );
    assert!(checker.type_work.is_none());
}

#[test]
pub(crate) fn field_hints_keep_metadata_record_types_and_selected_errors() {
    for (source, expected) in [
        (r#"p:@"proof";alias:p;kind:(alias.revision)<>"#, (2, 1)),
        (r#"kind:((@"proof").revision)<>"#, (2, 1)),
        ("record:{->n<uint32>:7};kind:record.n<>", (3, 2)),
    ] {
        assert_eq!(cost(source), expected, "{source}");
        crate::compile(source).unwrap();
    }
    for (source, code, token) in [
        ("kind:<({-><int32[1/0]>})>", "E107", "1/0"),
        (
            "kind:(<({-><int32[1/0]>})>.missing)<>",
            "B001",
            "(<({-><int32[1/0]>})>.missing)<>",
        ),
        ("kind:(7~<int32[1/0]>)<>", "B001", "(7~<int32[1/0]>)<>"),
    ] {
        let error = crate::compile(source).unwrap_err().remove(0);
        assert_eq!(error.code, code, "{source}");
        assert_eq!(&source[error.span.start..error.span.end], token);
    }
}
