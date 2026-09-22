use crate::check::Checker;

#[test]
pub(crate) fn indirect_stores_preserve_rhs_and_control_dependencies() {
    for store in ["*r=seed", "|seed|*r=false", "copy:&!*r;*copy=seed"] {
        let source = format!(
            "seed:false;x:=true;r:&!x;{store};flag:*r;p:@\"proof\";|flag|q:p.can_copy<uint32>()"
        );
        let mut checker = Checker::new();
        checker.derived.insert(0);
        let block = crate::parser::parse(&source).unwrap();
        let body = checker.block(&block, None, None).unwrap();
        assert!(checker.derived_local(1), "{store}");
        assert!(checker.derived_local(2), "{store}");
        assert!(checker.queries[0].control);
        let program = crate::hir::Program {
            body,
            functions: Vec::new(),
            locals: checker.locals.clone(),
        };
        checker.proofs.conditions = checker.guards.clone();
        checker.proofs.tags = checker.tags.clone();
        let facts = crate::borrow::check(&program, &mut checker.flow, &checker.proofs).unwrap();
        crate::loans::check(&program, &facts, &checker.proofs, &mut checker.flow).unwrap();
        assert_eq!(
            crate::check::queries::finish(&checker.queries, &checker.query_budgets)
                .unwrap_err()
                .code,
            "E225"
        );
    }
}

#[test]
pub(crate) fn indexed_store_targets_mark_the_known_owner() {
    let source = "index:1;items<boolean[2]>:=[false,false];r:&!(items[index]);*r=true;flag:items[2];p:@\"proof\";|flag|q:p.can_copy<uint32>()";
    let mut checker = Checker::new();
    checker.derived.insert(0);
    let block = crate::parser::parse(source).unwrap();
    checker.block(&block, None, None).unwrap();
    assert!(checker.derived_local(1));
    assert!(checker.queries[0].control);
}

#[test]
pub(crate) fn indirect_store_errors_preserve_unmodified_owner_marks() {
    for (source, code) in [
        ("seed:true;x:=false;r:&x;*r=seed", "E305"),
        ("seed:true;x:=7;r:&!x;*r=seed", "E207"),
        ("seed:true;x:=false;r:={->&!x};*r=seed", "B001"),
    ] {
        let mut checker = Checker::new();
        checker.derived.insert(0);
        let block = crate::parser::parse(source).unwrap();
        let error = checker.block(&block, None, None).unwrap_err();
        assert_eq!(error.code, code, "{source}: {error:?}");
        if code == "B001" {
            assert!(error.message.contains("indirect store origins"));
        }
        assert!(!checker.derived_local(1));
    }
}

#[test]
pub(crate) fn ordinary_indirect_stores_keep_unmarked_owners() {
    let source = "x:=false;r:=&!x;*r=true;flag:*r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let block = crate::parser::parse(source).unwrap();
    checker.block(&block, None, None).unwrap();
    assert!(checker.derived.is_empty());
}
