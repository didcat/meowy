use super::*;

pub(super) fn declare(checker: &mut Checker, source: &str) -> Result<hir::FunctionId> {
    let block = crate::parser::parse(source).unwrap();
    let ast::StmtKind::Bind {
        name, ty, value, ..
    } = &block.stmts[0].kind
    else {
        panic!()
    };
    let ExprKind::Function { params, body } = &value.kind else {
        panic!()
    };
    checker.declare_function(name, ty.as_ref(), params, body, block.stmts[0].span)
}

#[test]
pub(crate) fn function_declarations_return_their_exact_id_across_nested_definitions() {
    let mut checker = Checker::new();
    let first = declare(&mut checker, "first<int32>:(){->1}").unwrap();
    let outer = declare(
        &mut checker,
        "outer<int32>:(){inner<int32>:(){->2};->inner()}",
    )
    .unwrap();
    let next = declare(&mut checker, "next<int32>:(){->3}").unwrap();
    assert_eq!((first, outer, next), (0, 1, 3));
    for (name, id) in [("first", first), ("outer", outer), ("next", next)] {
        let function = checker.functions[id].as_ref().unwrap();
        assert_eq!(function.id, id);
        assert_eq!(function.name, name);
        assert_eq!(checker.bodies[&function.body.id].owner, id + 1);
    }
    assert_eq!(checker.functions[2].as_ref().unwrap().name, "inner");
    assert_eq!(checker.owner, 0);
    assert_eq!(checker.reach, TRUE);
}

#[test]
pub(crate) fn function_declarations_return_no_identity_for_rejected_bodies() {
    let mut checker = Checker::new();
    let error = declare(&mut checker, "bad<int32>:(){->false}").unwrap_err();
    assert_eq!(error.code, "E207");
    assert!(checker.functions[0].is_none());
}
