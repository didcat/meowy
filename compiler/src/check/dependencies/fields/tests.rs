use super::*;

pub(crate) fn check(source: &str) -> (Checker, hir::Block) {
    let mut checker = Checker::new();
    let block = checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    (checker, block)
}

#[test]
pub(crate) fn field_stages_keep_owned_and_shared_receivers_without_duplicate_loads() {
    for (source, load, explicit) in [
        ("r:{->z:1;->a:2};x:r.z", false, false),
        ("r:{->z:1;->a:2};p:&r;x:p.z", true, false),
        ("r:{->z:1};p:&r;x:(*p).z", false, true),
        ("n:1;r:{->z:&n};x:r.z", false, false),
    ] {
        crate::compile(source).unwrap();
        let (checker, block) = check(source);
        let (&id, field) = checker.fields.first_key_value().unwrap();
        assert_eq!(field.load, load);
        assert_eq!(!checker.derefs.is_empty(), explicit);
        let hir::Stmt::Bind { value, .. } = block.stmts.last().unwrap() else {
            panic!()
        };
        let hir::ExprKind::Field { index, .. } = value.kind else {
            panic!()
        };
        assert_eq!(field.index, index);
        assert_eq!(checker.points[field.input].parent, Some(id));
        assert_eq!(
            field.edges[0],
            Edge::new(Port::Entry(id), Port::Entry(field.input), Route::Next)
        );
        let mut from = Port::Normal(field.input);
        if load {
            let stage = Port::Projection { point: id, step: 0 };
            assert_eq!(field.edges[1], Edge::new(from, stage, Route::Next));
            from = stage;
        }
        assert_eq!(
            field.edges[field.edges.len() - 2],
            Edge::new(from, Port::Operation(id), Route::Next)
        );
        assert_eq!(
            field.edges.last().unwrap(),
            &Edge::new(Port::Operation(id), Port::Normal(id), Route::Next)
        );
        assert!(!checker.region_edges.contains_key(&id));
    }
}

#[test]
pub(crate) fn field_stages_preserve_call_returns_nested_fields_and_narrowing() {
    let source = "f<{z<int32>}>:(){->{->z:3}};x:f().z;r:{->i:{->z:4}};y:r.i.z";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let call = checker.invocations.values().next().unwrap();
    let field = checker
        .fields
        .values()
        .find(|field| field.input == call.point)
        .unwrap();
    assert!(!field.load);
    assert_eq!(call.edges.last().unwrap().route, Route::Returned);
    assert_eq!(
        checker
            .fields
            .values()
            .filter(|field| checker.fields.contains_key(&field.input))
            .count(),
        1
    );
    let source = "f<null>:(r<{n<int32><null>}>){|r.n<int32>|{v<int32>:r.n}}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    assert_eq!(checker.fields.len(), 2);
    assert!(
        checker
            .fields
            .values()
            .all(|field| field.owner != 0 && field.normal)
    );
    let source = "flag:false;r:{->n:1};|flag|v:r.n";
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    assert!(checker.fields.values().next().unwrap().control);
}

#[test]
pub(crate) fn field_stages_preserve_never_fields_early_exits_and_failures() {
    for source in [
        "f<never>:(r<{n<never>}>){->r.n}",
        "f<never>:(r<&{n<never>}>){->r.n}",
    ] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        let (&id, field) = checker.fields.first_key_value().unwrap();
        assert!(!field.normal);
        assert!(!field.edges.iter().any(|edge| edge.to == Port::Normal(id)));
    }
    let (checker, _) = check(
        "p:@\"proof\";v:p.revision;r:{->n:3};<T>:{-><uint8[r.n]>};q:p.can_copy<uint8>();<F>:q.always<>",
    );
    assert!(checker.fields.is_empty());
    for (source, code) in [
        ("r:{->n:1};v:r.missing", "E201"),
        ("v:(1).n", "E201"),
        ("d:@\"debug\";v:d.panic(\"stop\").n", "E201"),
        ("p:@\"proof\";q:p.can_copy<uint8>();v:q.always", "B001"),
    ] {
        let mut checker = Checker::new();
        assert_eq!(
            checker
                .block(&crate::parser::parse(source).unwrap(), None, None)
                .unwrap_err()
                .code,
            code,
            "{source}"
        );
        assert!(checker.fields.is_empty());
        assert!(checker.point.is_none());
    }
}

#[test]
pub(crate) fn field_stages_bound_type_validation_and_atomic_publication() {
    let (mut checker, block) = check("r:{->n:1};v:r.n");
    let hir::Stmt::Bind { value, .. } = &block.stmts[1] else {
        panic!()
    };
    let (&id, field) = checker.fields.first_key_value().unwrap();
    let field = field.clone();
    let count = checker.field_edges;
    checker
        .field_operation(id, field.input, field.load, value, field.span)
        .unwrap();
    assert_eq!(checker.field_edges, count);
    checker.fields.clear();
    checker.field_edges = 0;
    for case in 0..4 {
        let mut value = value.clone();
        match case {
            0 => value.ty = hir::Type::Bool,
            1 => {
                let hir::ExprKind::Field { index, .. } = &mut value.kind else {
                    panic!()
                };
                *index = usize::MAX;
            }
            2 => checker.points[field.input].parent = None,
            _ => (),
        }
        assert!(
            checker
                .field_operation(id, field.input, case == 3, &value, field.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        checker.points[field.input].parent = Some(id);
    }
    let mut large = value.clone();
    large.ty = hir::Type::Record {
        primary: Box::new(hir::Type::Null),
        fields: (0..=crate::borrow_value::MAX_PARTS)
            .map(|i| hir::Field {
                name: i.to_string(),
                ty: hir::Type::Bool,
                mutable: false,
            })
            .collect(),
    };
    assert!(
        checker
            .field_operation(id, field.input, false, &large, field.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    checker.reborrow_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .field_operation(id, field.input, false, value, field.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.fields.is_empty());
    assert_eq!(checker.field_edges, 0);
}
