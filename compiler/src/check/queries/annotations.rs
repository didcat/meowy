use super::accounting::checker;
use crate::ast::Span;
use crate::check::required::{MAX_STEPS, MAX_TYPES};

#[test]
pub(crate) fn pending_annotations_charge_aliases_and_copies_without_requerying() {
    for annotation in ["p.Result", "R"] {
        let mut checker = checker();
        let block = crate::parser::parse(&format!(
            "r:p.can_copy<uint32>();<R>:r<>;copy<{annotation}>:r"
        ))
        .unwrap();
        checker.stmt(&block.stmts[0]).unwrap();
        checker.stmt(&block.stmts[1]).unwrap();
        let root = Span::new(100, 150);
        checker
            .construction_root(root, |checker| {
                checker.stmt(&block.stmts[2])?;
                let work = checker.type_work.as_ref().unwrap();
                assert_eq!((work.logical.steps, work.logical.types), (1, 1));
                assert_eq!(work.logical.root, root);
                assert!(work.ordinary);
                Ok(())
            })
            .unwrap();
        assert_eq!(checker.queries.len(), 1);
        assert_eq!(checker.query_budgets.len(), 1);
        assert!(checker.type_work.is_none());
        assert!(checker.locals.is_empty());
    }
}

#[test]
pub(crate) fn pending_annotations_share_outer_limits_and_restore_roots() {
    for (steps, types, code) in [
        (MAX_STEPS - 3, MAX_TYPES - 2, None),
        (MAX_STEPS - 2, 0, Some("E220")),
        (0, MAX_TYPES - 1, Some("E220")),
    ] {
        let mut checker = checker();
        let block = crate::parser::parse("r<p.Result>:p.can_copy<uint32>()").unwrap();
        let root = Span::new(100, 150);
        let result = checker.construction_root(root, |checker| {
            checker
                .type_work
                .as_mut()
                .unwrap()
                .logical
                .charge(steps, types)?;
            checker.stmt(&block.stmts[0])?;
            Ok(())
        });
        assert_eq!(result.as_ref().err().map(|error| error.code), code);
        let budget = checker.query_budgets[0].as_ref().unwrap();
        assert_eq!(budget.root, root);
        if let Err(error) = result {
            assert_eq!(error.span, root);
            assert_eq!(budget.failure.as_ref().unwrap().code, "E220");
        } else {
            assert_eq!((budget.steps, budget.types), (MAX_STEPS, MAX_TYPES));
        }
        assert!(checker.type_work.is_none());
        let next = crate::parser::parse("next<p.Result>:p.can_copy<uint32>()").unwrap();
        checker.stmt(&next.stmts[0]).unwrap();
        assert!(checker.type_work.is_none());
    }
}

#[test]
pub(crate) fn pending_annotations_preserve_constructor_errors_and_work() {
    for (annotation, code, steps, types) in [
        ("Missing", "E202", 0, 0),
        ("p.Always", "E207", 1, 1),
        ("int32", "E223", 1, 1),
        ("uint8[1/0]", "E107", 5, 2),
    ] {
        let mut checker = checker();
        let block =
            crate::parser::parse(&format!("r:p.can_copy<uint32>();copy<{annotation}>:r")).unwrap();
        checker.stmt(&block.stmts[0]).unwrap();
        checker
            .construction_root(block.span, |checker| {
                let error = checker.stmt(&block.stmts[1]).unwrap_err();
                assert_eq!(error.code, code, "{annotation}");
                let work = checker.type_work.as_ref().unwrap();
                assert_eq!((work.logical.steps, work.logical.types), (steps, types));
                Ok(())
            })
            .unwrap();
        assert!(checker.type_work.is_none());
        assert_eq!(checker.queries.len(), 1);
    }
}
