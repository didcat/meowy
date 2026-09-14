use crate::ast::Span;
use crate::check::{Checker, Result, required::MAX_STEPS};
use crate::hir::Type;

pub(crate) fn cost(source: &str) -> (usize, usize) {
    let block = crate::parser::parse(source).unwrap();
    let mut checker = Checker::new();
    checker
        .required_root(block.span, |checker| {
            checker.required_block(
                &block,
                Some(&Type::Int {
                    bits: 32,
                    signed: true,
                }),
            )?;
            let budget = &checker.type_work.as_ref().unwrap().logical;
            Ok((budget.steps, budget.types))
        })
        .unwrap()
}

#[test]
pub(crate) fn logical_statement_steps_count_blocks_once_and_preserve_groups() {
    assert_eq!(cost("a:1;b:2;->3"), (4, 0));
    assert_eq!(cost("->7"), (2, 0));
    assert_eq!(cost("->(((7)))"), (2, 0));
    assert_eq!(cost("->{->7}"), (4, 0));
    assert_eq!(cost("->(({->7}))"), (4, 0));
    assert_eq!(cost("<T>:<uint8>;->7"), (4, 1));
}

#[test]
pub(crate) fn logical_statement_steps_skip_bodies_but_charge_selected_statements() {
    let skipped = cost("|false|x:1;->7");
    let selected = cost("|true|x:1;->7");
    assert_eq!(selected.0, skipped.0 + 1);
    assert_eq!(selected.1, skipped.1);
    assert_eq!(cost("|false|x:<int32[4]>;->7"), skipped);
}

#[test]
pub(crate) fn logical_statement_limits_restore_scope_and_stop_before_later_statements() {
    let block = crate::parser::parse("a:1;b:2;->3").unwrap();
    let root = Span::new(100, 140);
    for remaining in [3, 4, 5] {
        let mut checker = Checker::new();
        let scopes = checker.scopes.len();
        let result = checker.required_root(root, |checker| {
            checker.type_work.as_mut().unwrap().logical.steps = MAX_STEPS - remaining;
            checker.required_block(
                &block,
                Some(&Type::Int {
                    bits: 32,
                    signed: true,
                }),
            )
        });
        assert_eq!(result.is_ok(), remaining >= 4);
        if let Err(error) = result {
            assert_eq!(error.code, "E220");
            assert_eq!(error.span, root);
        }
        assert_eq!(checker.scopes.len(), scopes);
        assert!(checker.type_work.is_none());
    }
    let source = "a:1/0;tail:missing;->3";
    let block = crate::parser::parse(source).unwrap();
    let mut checker = Checker::new();
    let result: Result<()> = checker.required_root(block.span, |checker| {
        let result = checker.required_block(
            &block,
            Some(&Type::Int {
                bits: 32,
                signed: true,
            }),
        );
        assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, 2);
        result.map(|_| ())
    });
    let error = result.unwrap_err();
    assert_eq!(error.code, "E107");
    assert_eq!(&source[error.span.start..error.span.end], "1/0");
    assert!(checker.type_work.is_none());
}
