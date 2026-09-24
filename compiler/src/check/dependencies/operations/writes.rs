use super::{tests::check, *};

#[test]
pub(crate) fn write_operations_keep_distinct_values_and_one_storage_target() {
    let source = "x:=1;x=2;x=3";
    crate::compile(source).unwrap();
    let checker = check(source);
    let writes = checker
        .operations
        .iter()
        .filter(|(_, op)| op.kind == Kind::Write)
        .collect::<Vec<_>>();
    assert_eq!(writes.len(), 2);
    assert_eq!(writes[0].1.storage, writes[1].1.storage);
    assert_ne!(writes[0].1.input, writes[1].1.input);
    for (id, op) in writes {
        let input = op.input.unwrap();
        assert_eq!(checker.points[input].parent, Some(*id));
        assert!(op.edges.contains(&Edge::new(
            Port::Normal(input),
            Port::Operation(*id),
            Route::Next
        )));
        assert!(op.edges.contains(&Edge::new(
            Port::Operation(*id),
            Port::Normal(*id),
            Route::Next
        )));
    }
}

#[test]
pub(crate) fn write_operations_resolve_slot_aliases_without_confusing_reference_cells() {
    let source = "c:=false;row:'out{|c|{'out->value:=1;value=2};|!c|{'out->value:=1;value=3}}";
    crate::compile(source).unwrap();
    let checker = check(source);
    let writes = checker
        .operations
        .values()
        .filter(|op| op.kind == Kind::Write)
        .collect::<Vec<_>>();
    assert_eq!(writes.len(), 2);
    assert_ne!(writes[0].local, writes[1].local);
    assert_eq!(writes[0].storage, writes[1].storage);
    for op in writes {
        assert_eq!(op.storage, checker.proofs.aliases[&op.local].root);
    }
    let source = "a:1;b:2;r:=&a;r=&b;copy:*r";
    crate::compile(source).unwrap();
    let checker = check(source);
    let op = checker
        .operations
        .values()
        .find(|op| op.kind == Kind::Write)
        .unwrap();
    assert_eq!(op.local, op.storage);
    assert!(matches!(
        checker.locals[op.storage],
        hir::Type::Reference(_)
    ));
}

#[test]
pub(crate) fn write_operations_keep_control_marks_and_original_type_mutability_loan_errors() {
    let mut checker = Checker::new();
    checker.derived.insert(0);
    let block = crate::parser::parse("flag:false;x:=1;|flag|x=2").unwrap();
    checker.block(&block, None, None).unwrap();
    let op = checker
        .operations
        .values()
        .find(|op| op.kind == Kind::Write)
        .unwrap();
    assert!(op.control);
    for (source, code) in [("x:1;x=2", "E305"), ("x:=1;x=false", "E207")] {
        let mut checker = Checker::new();
        let block = crate::parser::parse(source).unwrap();
        assert_eq!(checker.block(&block, None, None).unwrap_err().code, code);
        assert!(!checker.operations.values().any(|op| op.kind == Kind::Write));
    }
    assert_eq!(
        crate::compile("x:=1;r:&x;x=2;after:*r").unwrap_err()[0].code,
        "E302"
    );
}

#[test]
pub(crate) fn write_operations_leave_path_and_indirect_stores_outside_direct_storage_links() {
    let source = "row:{->x:=1};row.x=2;xs<int32[2]>:=[1,2];xs[1]=3;x:=1;r:&!x;*r=2";
    crate::compile(source).unwrap();
    let checker = check(source);
    assert!(!checker.operations.values().any(|op| op.kind == Kind::Write));
}
