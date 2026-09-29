use super::{super::tests::check, *};

#[test]
pub(crate) fn binary_types_preserve_scalar_operands_and_comparison_results() {
    for (source, input, result) in [
        (
            "a<int8>:=7;b<int8>:=2;x:a+b",
            ScalarKind::Int {
                bits: 8,
                signed: true,
            },
            ScalarKind::Int {
                bits: 8,
                signed: true,
            },
        ),
        (
            "x:1.25<2.5",
            ScalarKind::Float { bits: 64 },
            ScalarKind::Bool,
        ),
        ("x:false==true", ScalarKind::Bool, ScalarKind::Bool),
        ("x:null==null", ScalarKind::Null, ScalarKind::Bool),
        ("x:\"a\"<\"b\"", ScalarKind::String, ScalarKind::Bool),
    ] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        let op = checker.binaries.values().next().unwrap();
        assert_eq!(
            op.types,
            Types {
                inputs: [Class::Scalar(input); 2],
                result: Class::Scalar(result)
            }
        );
    }
}

#[test]
pub(crate) fn binary_types_keep_stopped_and_nonscalar_operands_distinct() {
    let source = "f<never>:(r<{-><never>;tag<boolean>}>){->r+1};a:{->n:1};b:{->n:2};x:a==b";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    assert_eq!(checker.binaries.len(), 2);
    for op in checker.binaries.values() {
        if op.owner == 0 {
            assert_eq!(
                op.types,
                Types {
                    inputs: [Class::Other; 2],
                    result: Class::Scalar(ScalarKind::Bool)
                }
            );
        } else {
            assert_eq!(op.types.inputs[0], Class::Never);
            assert_eq!(op.types.result, Class::Never);
            assert_eq!(op.plan.normal, [false, true]);
        }
    }
    for source in [
        "xs:[1];ys:[2];x:xs==ys",
        "n:1;a:&n;b:&n;x:a==b",
        "a<int32><null>:1;b<int32><null>:null;x:a==b",
    ] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        assert_eq!(
            checker.binaries.values().next().unwrap().types.inputs,
            [Class::Other; 2]
        );
    }
}

#[test]
pub(crate) fn binary_types_reject_republication_with_different_scalar_widths() {
    let (mut checker, body) = check("1+2");
    let crate::hir::Stmt::Expr(value) = &body.stmts[0] else {
        panic!()
    };
    let (&id, op) = checker.binaries.first_key_value().unwrap();
    let op = op.clone();
    let counts = checker.edge_counts();
    checker
        .binary_operation(id, op.inputs, op.plan, value, op.span)
        .unwrap();
    let mut value = value.clone();
    let ty = Type::Int {
        bits: 16,
        signed: true,
    };
    let crate::hir::ExprKind::Binary { left, right, .. } = &mut value.kind else {
        panic!()
    };
    left.ty = ty.clone();
    right.ty = ty.clone();
    value.ty = ty;
    assert!(
        checker
            .binary_operation(id, op.inputs, op.plan, &value, op.span)
            .is_err()
    );
    assert_eq!(checker.binaries[&id], op);
    assert_eq!(checker.edge_counts(), counts);
}
