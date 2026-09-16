use crate::ast::Span;
use crate::check::Checker;

#[test]
pub(crate) fn checked_signatures_reuse_parameter_types_before_body_shadowing() {
    for source in [
        "n:2;f<int32>:(n<int32>,items<int32[n]>){->items[1]};v:f(3,[7,9])",
        "n:2;f<(int32,int32[2])->int32>;f<int32>:(n<int32>,items<int32[n]>){->items[1]};v:f(3,[7,9])",
    ] {
        let program = crate::compile(source).unwrap();
        let function = &program.functions[0];
        assert_eq!(
            program.locals[function.params[1]],
            crate::hir::Type::List {
                element: Box::new(crate::hir::Type::Int {
                    bits: 32,
                    signed: true
                }),
                capacity: 2,
            }
        );
    }
    for (source, code) in [
        ("f<int32>:(n<int32>,n<int32>){->n}", "E203"),
        ("f<int32>:(items<int32[1/0]>){->0}", "E107"),
        ("f<(int32)->int32>;f<int32>:(n<uint8>){->0}", "E221"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn checked_signatures_do_not_replay_computed_parameter_work() {
    let block = crate::parser::parse("f<int32>:(n<({-><int32>})>){->n}").unwrap();
    let mut checker = Checker::new();
    checker
        .required_root(Span::new(100, 150), |checker| {
            checker.block(&block, None, None)?;
            let work = checker.type_work.as_ref().unwrap();
            assert_eq!((work.visits, work.nodes), (3, 2));
            assert_eq!((work.logical.steps, work.logical.types), (7, 4));
            Ok(())
        })
        .unwrap();
}

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
pub(crate) fn signature_roots_charge_each_written_annotation_once() {
    for (source, expected) in [
        ("f<int32>:(n<int32>){->n}", (2, 2)),
        ("f:(n<int32>){->n}", (1, 1)),
        ("f<int32>:(n<int32><int32>){->n}", (4, 4)),
        ("f<int32>:(n<({-><int32>})>){->n}", (7, 4)),
        ("f<(int32)->int32>;f<int32>:(n<int32>){->n}", (5, 5)),
        ("f<(int32)->int32>;f:(n<int32>){->n}", (4, 4)),
        ("f<int32>:(n<int32>){->n};g<int32>:(n<int32>){->n}", (4, 4)),
        ("f<int32>:(items<int32[1+1]>){->items[1]}", (6, 3)),
    ] {
        assert_eq!(cost(source), expected, "{source}");
    }
}

#[test]
pub(crate) fn signature_roots_preserve_modes_and_source_order() {
    crate::compile("f<int32>:(a<({-><int32[({->2})]>})>,b<int32[2]>){->a[1]+b[1]}").unwrap();
    assert_eq!(
        crate::compile("f<int32>:(a<({-><int32>})>,b<int32[({->2})]>){->a}").unwrap_err()[0].code,
        "B001"
    );
    for source in [
        "f<int32[1/0]>:(n<int32[2/0]>){->n}",
        "f<(int32[2])->int32[2]>;f<int32[1/0]>:(n<int32[2/0]>){->n}",
    ] {
        let error = crate::compile(source).unwrap_err().remove(0);
        assert_eq!(error.code, "E107");
        assert_eq!(&source[error.span.start..error.span.end], "1/0");
    }
}

#[test]
pub(crate) fn signature_roots_share_limits_and_restore_after_failure() {
    let block = crate::parser::parse("f<int32>:(n<int32>){->n}").unwrap();
    let root = Span::new(100, 150);
    for remaining in [1, 2, 3] {
        let mut checker = Checker::new();
        let result = checker.mode_root(root, true, |checker| {
            checker.type_work.as_mut().unwrap().logical.types =
                crate::check::required::MAX_TYPES - remaining;
            let result = checker.stmt(&block.stmts[0]);
            assert!(checker.type_work.as_ref().unwrap().ordinary);
            result
        });
        assert_eq!(result.is_ok(), remaining >= 2);
        if let Err(error) = result {
            assert_eq!(error.code, "E220");
            assert_eq!(error.span, root);
            assert!(checker.functions.is_empty());
        }
        assert!(checker.type_work.is_none());
        let mut next = Checker::new();
        next.block(&block, None, None).unwrap();
        assert!(next.type_work.is_none());
    }
}

#[test]
pub(crate) fn signature_exports_charge_explicit_signatures_without_body_replay() {
    assert_eq!(cost("->f<int32>:(n<int32>){->n}"), (2, 2));
    assert_eq!(
        cost("f<int32>:(n<int32>){->n};->g<(int32)->int32>:f"),
        (5, 5)
    );
    assert_eq!(
        cost("<F>:<(int32)->int32>;f<int32>:(n<int32>){->n};->g<F>:f"),
        (8, 8)
    );
    assert_eq!(
        crate::compile("f<int32>:(n<int32>){->n};->g<(uint8)->int32>:f").unwrap_err()[0].code,
        "E207"
    );
}
