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
    assert_eq!(cost("a:1;b:2;->3"), (7, 0));
    assert_eq!(cost("->7"), (3, 0));
    assert_eq!(cost("->(((7)))"), (3, 0));
    assert_eq!(cost("->{->7}"), (5, 0));
    assert_eq!(cost("->(({->7}))"), (5, 0));
    assert_eq!(cost("<T>:<uint8>;->7"), (5, 1));
}

#[test]
pub(crate) fn logical_statement_steps_skip_bodies_but_charge_selected_statements() {
    let skipped = cost("|false|x:1;->7");
    let selected = cost("|true|x:1;->7");
    assert_eq!(selected.0, skipped.0 + 2);
    assert_eq!(selected.1, skipped.1);
    assert_eq!(cost("|false|x:<int32[4]>;->7"), skipped);
}

#[test]
pub(crate) fn logical_statement_limits_restore_scope_and_stop_before_later_statements() {
    let block = crate::parser::parse("a:1;b:2;->3").unwrap();
    let root = Span::new(100, 140);
    for remaining in [6, 7, 8] {
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
        assert_eq!(result.is_ok(), remaining >= 7);
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
        assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, 5);
        result.map(|_| ())
    });
    let error = result.unwrap_err();
    assert_eq!(error.code, "E107");
    assert_eq!(&source[error.span.start..error.span.end], "1/0");
    assert!(checker.type_work.is_none());
}

pub(crate) fn boolean_cost(source: &str, checker: &mut Checker) -> Result<usize> {
    let block = crate::parser::parse(&format!("value:{source}")).unwrap();
    let crate::ast::StmtKind::Bind { value, .. } = &block.stmts[0].kind else {
        panic!()
    };
    checker.required_root(value.span, |checker| {
        checker.type_boolean(value, None)?;
        Ok(checker.type_work.as_ref().unwrap().logical.steps)
    })
}

#[test]
pub(crate) fn logical_boolean_steps_exclude_groups_and_skipped_operands() {
    for (source, steps) in [
        ("true", 1),
        ("(((true)))", 1),
        ("!false", 2),
        ("true||false", 2),
        ("false&&true", 2),
        ("true&&false", 3),
        ("false||true", 3),
        ("true==false", 3),
        ("true!=false", 3),
        ("true&&(true==false)", 5),
        ("false&&(true==false)", 2),
        ("true||({->false})", 2),
        ("false||({->true})", 5),
        ("({->true})", 3),
        ("(({->true}))", 3),
    ] {
        assert_eq!(
            boolean_cost(source, &mut Checker::new()).unwrap(),
            steps,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn logical_boolean_form_checks_do_not_spend_steps() {
    let block = crate::parser::parse("value:true||missing").unwrap();
    let crate::ast::StmtKind::Bind { value, .. } = &block.stmts[0].kind else {
        panic!()
    };
    let mut checker = Checker::new();
    let result: Result<()> = checker.required_root(value.span, |checker| {
        let result = checker.type_boolean(value, None);
        assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, 0);
        result.map(|_| ())
    });
    assert_eq!(result.unwrap_err().code, "E201");
}

#[test]
pub(crate) fn logical_boolean_limits_restore_depth_and_independent_roots() {
    let block = crate::parser::parse("value:true&&false").unwrap();
    let crate::ast::StmtKind::Bind { value, .. } = &block.stmts[0].kind else {
        panic!()
    };
    let root = Span::new(100, 150);
    for remaining in [2, 3, 4] {
        let mut checker = Checker::new();
        let result = checker.required_root(root, |checker| {
            checker.type_work.as_mut().unwrap().logical.steps = MAX_STEPS - remaining;
            let result = checker.type_boolean(value, None);
            assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
            result
        });
        assert_eq!(result.is_ok(), remaining >= 3);
        if let Err(error) = result {
            assert_eq!(error.code, "E220");
            assert_eq!(error.span, root);
        }
        assert!(checker.type_work.is_none());
        assert_eq!(boolean_cost("true&&false", &mut checker).unwrap(), 3);
    }
}

#[test]
pub(crate) fn logical_boolean_reads_do_not_reuse_bootstrap_initializer_counts() {
    for work in [7, 100] {
        let mut checker = crate::check::inputs::tests::check("flag:{->false;unused:7}");
        let id = *checker.bool_inputs.keys().last().unwrap();
        checker.bool_inputs.get_mut(&id).unwrap().work = work;
        checker
            .declare(
                "flag",
                crate::check::Value::Local {
                    id,
                    ty: Type::Bool,
                    mutable: false,
                    owner: 0,
                    constant: None,
                },
                Span::new(0, 4),
            )
            .unwrap();
        assert_eq!(boolean_cost("flag==flag", &mut checker).unwrap(), 3);
        checker.bool_inputs.get_mut(&id).unwrap().error = Some(Checker::error(
            "E107",
            "original input failure",
            Span::new(17, 20),
        ));
        assert_eq!(boolean_cost("true||flag", &mut checker).unwrap(), 2);
        let error = boolean_cost("flag==flag", &mut checker).unwrap_err();
        assert_eq!(error.code, "E107");
        assert_eq!(error.span, Span::new(17, 20));
    }
}

#[test]
pub(crate) fn logical_control_and_boolean_blocks_combine_without_duplicate_charges() {
    assert_eq!(cost("flag:true;|flag|x:1;->7"), (9, 0));
    assert_eq!(cost("flag:false;|flag|x:1;->7"), (7, 0));
    assert_eq!(cost("flag:{->true};|flag|x:1;->7"), (11, 0));
    assert_eq!(cost("flag:(({->true}));|flag|x:1;->7"), (11, 0));
    assert_eq!(
        boolean_cost("({->true})==({->false})", &mut Checker::new()).unwrap(),
        7
    );
    assert_eq!(
        boolean_cost("!(({->false}))", &mut Checker::new()).unwrap(),
        4
    );
}

#[test]
pub(crate) fn logical_nested_boolean_failures_keep_outer_root_and_block_scope() {
    let block = crate::parser::parse("flag:!({->true});->7").unwrap();
    let root = Span::new(200, 240);
    let mut checker = Checker::new();
    let scopes = checker.scopes.len();
    let error = checker
        .required_root(root, |checker| {
            checker.type_work.as_mut().unwrap().logical.steps = MAX_STEPS - 4;
            checker
                .required_block(
                    &block,
                    Some(&Type::Int {
                        bits: 32,
                        signed: true,
                    }),
                )
                .map(|_| ())
        })
        .unwrap_err();
    assert_eq!(error.code, "E220");
    assert_eq!(error.span, root);
    assert_eq!(checker.scopes.len(), scopes);
    assert!(checker.type_work.is_none());
    assert_eq!(boolean_cost("!({->true})", &mut checker).unwrap(), 4);
}
