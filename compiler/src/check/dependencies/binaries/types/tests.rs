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
                    inputs: [Class::Record { fields: 1 }; 2],
                    result: Class::Scalar(ScalarKind::Bool)
                }
            );
        } else {
            assert_eq!(op.types.inputs[0], Class::Never);
            assert_eq!(op.types.result, Class::Never);
            assert_eq!(op.plan.normal, [false, true]);
        }
    }
    for (source, class) in [
        ("xs:[1];ys:[2];x:xs==ys", Class::List { capacity: 1 }),
        (
            "n:1;a:&n;b:&n;x:a==b",
            Class::Reference(ReferenceMode::Shared),
        ),
        (
            "a<int32><null>:1;b<int32><null>:null;x:a==b",
            Class::Union { members: 2 },
        ),
    ] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        assert_eq!(
            checker.binaries.values().next().unwrap().types.inputs,
            [class; 2]
        );
    }
}

#[test]
pub(crate) fn binary_types_capture_checked_equality_without_inventing_value_answers() {
    for source in [
        "a:{->7;->n:1};b:{->7;->n:2};x:a==b",
        "a<int32[3]>:[1];b<int32[3]>:[1,2];x:a!=b",
        "m:@\"memory\";a:m.heap;b:m.heap;x:(&a)==(&b)",
        "r:{->1;->tag:true};x:r==1",
        "a<int32><null>:1;b<int32><null>:null;x:a==b",
    ] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        let op = checker.binaries.values().next().unwrap();
        assert!(op.plan.equality && !op.plan.checked);
        assert_eq!(op.types.result, Class::Scalar(ScalarKind::Bool));
    }
    for source in [
        "1+2",
        "1<2",
        "f<never>:(r<{-><never>;tag<boolean>}>){->r==[1]}",
    ] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        assert!(!checker.binaries.values().next().unwrap().plan.equality);
    }
}

#[test]
pub(crate) fn binary_types_bound_categories_and_preserve_opaque_nominal_values() {
    for count in [0, 1, 2, MAX_COUNT, MAX_COUNT + 1] {
        let fields = vec![
            crate::hir::Field {
                name: "x".into(),
                ty: Type::Bool,
                mutable: false
            };
            count
        ];
        let record = Type::Record {
            primary: Box::new(Type::Null),
            fields,
        };
        assert_eq!(
            Class::of(&record),
            if count <= MAX_COUNT {
                Class::Record { fields: count }
            } else {
                Class::Other
            }
        );
        let list = Type::List {
            element: Box::new(Type::Bool),
            capacity: count,
        };
        assert_eq!(
            Class::of(&list),
            if count <= crate::list::MAX_CAPACITY {
                Class::List { capacity: count }
            } else {
                Class::Other
            }
        );
        let union = Type::Union(vec![Type::Bool; count]);
        assert_eq!(
            Class::of(&union),
            if (2..=MAX_COUNT).contains(&count) {
                Class::Union { members: count }
            } else {
                Class::Other
            }
        );
    }
    let heap = Type::Foundation(crate::hir::FoundationType::Allocator);
    assert_eq!(Class::of(&heap), Class::Other);
    assert_eq!(
        Class::of(&Type::Exclusive(Box::new(heap))),
        Class::Reference(ReferenceMode::Exclusive)
    );
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

#[test]
pub(crate) fn binary_types_preserve_checked_equality_on_replay() {
    let (mut checker, body) = check("a:[1];b:[2];a==b");
    let crate::hir::Stmt::Expr(value) = body.stmts.last().unwrap() else {
        panic!()
    };
    let (&id, op) = checker.binaries.first_key_value().unwrap();
    let op = op.clone();
    let counts = checker.edge_counts();
    let mut plan = op.plan;
    assert!(plan.equality);
    plan.equality = false;
    assert!(
        checker
            .binary_operation(id, op.inputs, plan, value, op.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(checker.binaries[&id], op);
    assert_eq!(checker.edge_counts(), counts);
}
