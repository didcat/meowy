use crate::check::{Checker, Constant, Value};

pub(crate) fn checker() -> Checker {
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
}

#[test]
pub(crate) fn controlled_bindings_keep_marks_through_nested_ordinary_conditions() {
    let mut checker = checker();
    let source = "flag:false;|flag|{n:3;b:true;row:{->n:4};|true|inner:5};plain:6";
    let block = crate::parser::parse(source).unwrap();
    checker.block(&block, None, None).unwrap();
    assert!(!checker.control);
    let plain = checker.locals.len() - 1;
    assert!(!checker.derived_local(plain));
    assert_eq!(checker.inputs[&plain].value, Some(6));
    assert_eq!(checker.derived.len(), 5);
    assert!(
        checker
            .inputs
            .iter()
            .filter(|(id, _)| **id != plain)
            .all(|(_, input)| input.derived)
    );
    assert!(checker.bool_inputs.values().all(|input| input.derived));
    assert!(checker.record_inputs.values().all(|row| row.input.derived));
}

#[test]
pub(crate) fn controlled_bindings_cannot_supply_required_type_values() {
    for tail in [
        "n:3;<T>:{-><uint8[n]>}",
        "b:true;<T>:{|b|-><uint8>;|!b|-><uint16>}",
        "row:{->n:3};<T>:{copy:row;-><uint8[copy.n]>}",
    ] {
        let mut checker = checker();
        let source = format!("flag:false;|flag|{{{tail}}}");
        let block = crate::parser::parse(&source).unwrap();
        let error = checker.block(&block, None, None).unwrap_err();
        assert_eq!(error.code, "E225", "{source}: {error:?}");
        assert!(!checker.control);
        assert!(checker.type_work.is_none());
    }
}

#[test]
pub(crate) fn controlled_binding_constants_and_error_scope_are_restored() {
    let mut checker = checker();
    let block = crate::parser::parse("flag:false;|flag|{n:3;bad<int32>:false}").unwrap();
    let error = checker.block(&block, None, None).unwrap_err();
    assert_eq!(error.code, "E207");
    assert!(!checker.control);
    assert_eq!(checker.scopes.len(), 2);
    assert!(
        checker
            .scopes
            .iter()
            .all(|scope| !scope.values.contains_key("n"))
    );
    let block = crate::parser::parse("plain:7").unwrap();
    checker.stmt(&block.stmts[0]).unwrap();
    let Value::Local { constant, id, .. } = &checker.scopes.last().unwrap().values["plain"] else {
        panic!()
    };
    assert!(matches!(constant, Some(Constant::Int(7))));
    assert!(!checker.derived_local(*id));
}
