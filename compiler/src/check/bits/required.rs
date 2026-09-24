use crate::ast::{Expr, StmtKind};
use crate::check::{Checker, Constant, Value};
use crate::foundation::{BitOp, Item};

pub(crate) fn checker() -> Checker {
    let mut checker = Checker::new();
    checker
        .declare(
            "combine",
            Value::Foundation(Item::Bits(BitOp::And)),
            crate::ast::Span::default(),
        )
        .unwrap();
    checker
}

pub(crate) fn expression(source: &str) -> Expr {
    let block = crate::parser::parse(&format!("value:{source}")).unwrap();
    let StmtKind::Bind { value, .. } = block.stmts.into_iter().next().unwrap().kind else {
        panic!("binding")
    };
    value
}

#[test]
pub(crate) fn bit_calls_retain_required_values_widths_blocks_and_dispatch() {
    for source in [
        "b:@\"bits\";<T>:{n<uint8>:b.and(255,3);flag:b.not(n)==252;|flag|-><uint8[n]>;|!flag|-><string>};v<T>:[1,2,3]",
        "b:@\"bits\";combine:b.or;<T>:{n<uint8>:1.(combine,2);-><int32[n]>};v<T>:[1,2,3]",
        "b:@\"bits\";<T>:{n<uint8>:b.and({v<uint8>:7;->v},{->3});-><int32[n]>};v<T>:[1,2,3]",
        "b:@\"bits\";<T>:{flag:b.and({->7},3)==3;|flag|-><int32>;|!flag|-><string>};v<T>:3",
        "b:@\"bits\";n:b.xor(7,4);<T>:<int32[n]>;v<T>:[1,2,3]",
    ] {
        crate::compile(source).unwrap();
    }
}

#[test]
pub(crate) fn required_bit_calls_keep_errors_eligibility_and_skipped_work() {
    for (body, code) in [
        ("n<uint8>:b.and(256,3)", "E216"),
        ("n:b.and(1/0,3)", "E107"),
        ("n:b.and(3,missing)", "E201"),
        ("n:b.or(3)", "E212"),
        ("n:b.and(true,false)", "B001"),
        ("a<uint8>:3;c<uint16>:4;n:b.or(a,c)", "E213"),
    ] {
        let source = format!("b:@\"bits\";<T>:{{{body};-><int32>}}");
        let error = crate::compile(&source).unwrap_err().remove(0);
        assert_eq!(error.code, code, "{body}: {error:?}");
    }
    crate::compile("b:@\"bits\";<T>:{flag:false&&(b.and(1/0,3)==0);-><int32>}").unwrap();
    let error =
        crate::compile("b:@\"bits\";f<int32>:(){->3};n:b.and(f(),7);<T>:{copy:n;-><int32>}")
            .unwrap_err()
            .remove(0);
    assert_eq!(error.code, "E211");
}

#[test]
pub(crate) fn bit_calls_charge_evaluated_nodes_once_and_restore_limits() {
    for (source, steps) in [("combine(7,3)", 3), ("combine({->7},3)", 5)] {
        let expr = expression(source);
        let mut checker = checker();
        checker
            .required_root(expr.span, |checker| {
                let result = checker.integer_arithmetic(&expr, None)?;
                assert!(matches!(
                    result,
                    Value::Static {
                        value: Constant::Int(3),
                        ..
                    }
                ));
                assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, steps);
                Ok(())
            })
            .unwrap();
        for remaining in [steps - 1, steps] {
            let result = checker.required_root(expr.span, |checker| {
                checker.type_work.as_mut().unwrap().logical.steps =
                    crate::check::required::MAX_STEPS - remaining;
                checker.integer_arithmetic(&expr, None)
            });
            assert_eq!(result.is_ok(), remaining == steps);
            if let Err(error) = result {
                assert_eq!(error.code, "E220");
            }
            assert!(checker.type_work.is_none());
            assert!(!checker.required);
        }
    }
}

#[test]
pub(crate) fn bit_calls_preserve_proof_marks_and_reject_derived_required_inputs() {
    let mut checker = checker();
    let block = crate::parser::parse("seed:7;copy:combine(seed,3);plain:combine(7,3)").unwrap();
    checker.stmt(&block.stmts[0]).unwrap();
    checker.inputs.get_mut(&0).unwrap().derived = true;
    checker.mark_derived(0);
    checker.stmt(&block.stmts[1]).unwrap();
    checker.stmt(&block.stmts[2]).unwrap();
    assert!(checker.inputs[&1].derived);
    assert!(checker.derived_local(1));
    assert!(!checker.inputs[&2].derived);
    let expr = expression("combine(copy,3)");
    let error = checker
        .required_root(expr.span, |checker| checker.type_scalar(&expr, None))
        .err()
        .unwrap();
    assert_eq!(error.code, "E225");
    assert!(checker.type_work.is_none());
}
