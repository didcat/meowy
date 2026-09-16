use crate::ast::{Span, StmtKind, TypeExpr};
use crate::check::{
    Checker,
    required::{MAX_STEPS, MAX_TYPES},
};

pub(crate) fn annotation(source: &str) -> TypeExpr {
    let block = crate::parser::parse(&format!("<A>:{source}")).unwrap();
    let StmtKind::TypeAlias { ty, .. } = &block.stmts[0].kind else {
        panic!()
    };
    ty.clone()
}

#[test]
pub(crate) fn ordinary_alias_roots_share_constructor_and_extent_costs() {
    for (source, expected) in [
        ("<int32><int32>", (3, 3)),
        ("<int32[1+2]>", (5, 2)),
        ("<int32[1+2]><int32[1+2]>", (11, 5)),
        ("<{a<int32[2]>;b<uint8[3]>}>", (8, 6)),
    ] {
        let ty = annotation(source);
        let root = Span::new(200, 250);
        let mut checker = Checker::new();
        checker
            .mode_root(root, true, |checker| {
                checker.declare_type("A", &ty, false, ty.span)?;
                let work = checker.type_work.as_ref().unwrap();
                assert_eq!(
                    (work.logical.steps, work.logical.types),
                    expected,
                    "{source}"
                );
                assert_eq!(work.logical.root, root);
                assert!(work.ordinary);
                Ok(())
            })
            .unwrap();
        assert!(checker.type_work.is_none());
    }
}

#[test]
pub(crate) fn ordinary_alias_roots_preserve_computed_mode_and_input_gates() {
    crate::compile("<A>:<{a<({-><int32[({->2})]>})>;b<int32[2]>}>").unwrap();
    let error = crate::compile("<A>:<{a<({-><int32>})>;b<int32[({->2})]>}>")
        .unwrap_err()
        .remove(0);
    assert_eq!(error.code, "B001");
    let ty = annotation("<int32[({->2})]>");
    let mut checker = Checker::new();
    assert_eq!(
        checker
            .declare_type("A", &ty, false, ty.span)
            .unwrap_err()
            .code,
        "B001"
    );
    assert!(checker.type_work.is_none());
    checker
        .required_root(ty.span, |checker| {
            checker.declare_type("A", &ty, false, ty.span)?;
            assert!(checker.proven_inputs());
            Ok(())
        })
        .unwrap();
    assert!(checker.type_work.is_none());
}

#[test]
pub(crate) fn ordinary_alias_roots_enforce_limits_and_reset_after_failures() {
    let ty = annotation("<int32[1+2]>");
    let root = Span::new(100, 150);
    for types in [false, true] {
        let total = if types { 2 } else { 5 };
        for remaining in [total - 1, total, total + 1] {
            let mut checker = Checker::new();
            let result = checker.mode_root(root, true, |checker| {
                let budget = &mut checker.type_work.as_mut().unwrap().logical;
                if types {
                    budget.types = MAX_TYPES - remaining;
                } else {
                    budget.steps = MAX_STEPS - remaining;
                }
                checker.declare_type("A", &ty, false, ty.span)
            });
            assert_eq!(result.is_ok(), remaining >= total);
            if let Err(error) = result {
                assert_eq!(error.code, "E220");
                assert_eq!(error.span, root);
                assert!(!checker.scopes.last().unwrap().types.contains_key("A"));
                checker.declare_type("A", &ty, false, ty.span).unwrap();
            }
            assert!(checker.type_work.is_none());
        }
    }
}

#[test]
pub(crate) fn ordinary_alias_roots_keep_failed_prefixes_and_lookup_isolation() {
    let ty = annotation("<int32[1/0]><Missing>");
    let mut checker = Checker::new();
    let error = checker
        .mode_root(ty.span, true, |checker| {
            let result = checker.declare_type("A", &ty, false, ty.span);
            let work = checker.type_work.as_ref().unwrap();
            assert_eq!((work.logical.steps, work.logical.types), (6, 3));
            assert!(work.ordinary);
            result
        })
        .unwrap_err();
    assert_eq!(error.code, "E107");
    assert!(checker.type_work.is_none());
    let ty = annotation("<int32><int32>");
    checker.spec(&ty).unwrap();
    assert!(checker.type_work.is_none());
    checker
        .mode_root(ty.span, true, |checker| {
            checker.spec(&ty)?;
            let budget = &checker.type_work.as_ref().unwrap().logical;
            assert_eq!((budget.steps, budget.types), (0, 0));
            Ok(())
        })
        .unwrap();
    checker.declare_type("A", &ty, false, ty.span).unwrap();
    checker.declare_type("B", &ty, false, ty.span).unwrap();
    assert!(checker.type_work.is_none());
}
