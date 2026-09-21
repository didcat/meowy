use super::accounting::{call, checker};
use crate::check::{Spec, Value};
use crate::hir::Type;

#[test]
pub(crate) fn pending_flag_signatures_share_logical_limits() {
    use crate::check::required::{MAX_STEPS, MAX_TYPES};

    for (steps, types, passes) in [
        (MAX_STEPS - 2, MAX_TYPES - 1, true),
        (MAX_STEPS - 1, 0, false),
        (MAX_STEPS, 0, false),
        (0, MAX_TYPES, false),
    ] {
        let mut checker = checker();
        let block = crate::parser::parse("r:p.can_copy<uint32>()").unwrap();
        checker.stmt(&block.stmts[0]).unwrap();
        let expr = call("kind:r.always<>");
        let result = checker.construction_root(expr.span, |checker| {
            checker
                .type_work
                .as_mut()
                .unwrap()
                .logical
                .charge(steps, types)?;
            checker.type_value(&expr)
        });
        assert_eq!(result.is_ok(), passes);
        if let Err(error) = result {
            assert_eq!(error.code, "E220");
            assert_eq!(error.span, expr.span);
        }
        assert!(checker.type_work.is_none());
        assert_eq!(checker.queries.len(), 1);
        assert!(checker.query_budgets[0].as_ref().unwrap().failure.is_none());
    }
}

#[test]
pub(crate) fn pending_flag_signatures_are_fixed_without_evaluating_answers() {
    for flag in ["always", "never", "indeterminable"] {
        let mut checker = checker();
        let source = format!(
            "r:p.can_copy<uint32>();copy:r;<Flag>:((copy).{flag})<>;kind:copy.{flag}<>;v<Flag>:false;next:p.can_copy<Flag>()"
        );
        let block = crate::parser::parse(&source).unwrap();
        checker.block(&block, None, None).unwrap();
        assert!(matches!(checker.queries[1].ty, Spec::Data(Type::Bool)));
        assert_eq!(checker.queries.len(), 2);
        assert_eq!(checker.locals.len(), 1);
    }
}

#[test]
pub(crate) fn pending_flag_signature_reads_do_not_prepare_or_charge_observations() {
    let mut checker = checker();
    let block = crate::parser::parse("r:p.can_copy<uint32>()").unwrap();
    checker.stmt(&block.stmts[0]).unwrap();
    let query = checker.query_budgets[0].as_ref().unwrap();
    let cost = (query.steps, query.types);
    for source in ["kind:r.always<>", "kind:(((r).always))<>"] {
        let expr = call(source);
        checker
            .construction_root(expr.span, |checker| {
                assert_eq!(checker.type_value(&expr)?, Type::Bool);
                let work = checker.type_work.as_ref().unwrap();
                assert_eq!((work.logical.steps, work.logical.types), (2, 1));
                Ok(())
            })
            .unwrap();
        assert_eq!(checker.queries.len(), 1);
        let query = checker.query_budgets[0].as_ref().unwrap();
        assert_eq!((query.steps, query.types), cost);
    }
    assert!(matches!(
        checker.symbol(&call("copy:r")).unwrap(),
        Some(Value::Pending(0))
    ));
}

#[test]
pub(crate) fn pending_flag_signatures_preserve_value_and_lookup_gates() {
    for (tail, code, message) in [
        ("flag:r.always", "B001", "pending proof scalar projections"),
        (
            "kind:r.missing<>",
            "B001",
            "type queries requiring expression evaluation",
        ),
        (
            "kind:r.always.missing<>",
            "B001",
            "type queries requiring expression evaluation",
        ),
        (
            "f:(){kind:r.always<>}",
            "B001",
            "type queries requiring expression evaluation",
        ),
        (
            "kind:(p.can_copy<uint32>()).always<>",
            "B001",
            "type queries requiring expression evaluation",
        ),
        ("kind:r.always<>;bad<int32>:false", "E207", ""),
        ("kind:r.always<>;x:=7;v:&x;x=8;copy:*v", "E302", ""),
    ] {
        let source = format!(r#"p:@"proof";r:p.can_copy<uint32>();{tail}"#);
        let error = crate::compile(&source).unwrap_err().remove(0);
        assert_eq!(error.code, code, "{source}: {error:?}");
        assert!(error.message.contains(message), "{source}: {error:?}");
    }
    crate::compile("r:{->always<uint32>:7};kind:r.always<>;v<(kind)>:7").unwrap();
}
