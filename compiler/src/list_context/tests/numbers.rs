use crate::ast::StmtKind;
use crate::check::Checker;
use crate::hir::Type;

use super::rejects;

#[test]
pub(crate) fn numeric_field_lists_keep_fixed_types_and_contexts() {
    for source in [
        "10:{->4<uint8>:2};v:[10.4,3];x<uint8>:v[1]",
        "10:{->4<uint8>:2};v:[3,10.4];x<uint8>:v[1]",
        "10:{->4<float32>:2.5};v:[10.4,3.5];x<float32>:v[1]",
        "10:{->4<float32>:2.5};v:[3.5,10.4];x<float32>:v[1]",
        "10:{->4<uint8>:2};v<uint8[2]><uint16[2]>:[10.4,3]",
        "10:{->4<uint8>:2};v<uint8[2]><uint16[2]>:[3,10.4]",
        "10:{->4<int8>:2};v:[-10.4,3];x<int8>:v[1]",
        "10:{->4<int8>:2};v:[3,-10.4];x<int8>:v[1]",
        "10:{->4<int8>:2};v<int8[1]><int16[1]>:[-10.4]",
        "10:{->4:{->n<uint8>:2}};v:[10.4,{->n<uint8>:3}]",
        "10:{->4:{->n<uint8>:2}};v:[{->n<uint8>:3},10.4]",
        "10:{->4:true};v<float32[1]>:[@\"core\".literal(10.4)]",
        "10:{->4:true};v<float32[1]>:[-@\"core\".literal(10.4)]",
        "v:[11.4,2.5];x<float64>:v[1]",
    ] {
        crate::compile(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
    }
    for (source, code) in [
        ("10:{->4<int32>:2};v<uint8[1]>:[10.4]", "E207"),
        ("10:{->4<uint8>:2};v<int32[1]>:[-10.4]", "E222"),
        ("10:{->4:2};v:[10.5]", "E201"),
        ("10:2;v:[10.4]", "E201"),
        ("10:{->4:2};v<float32[1]>:[10.5]", "E201"),
    ] {
        rejects(source, code);
    }
}

#[test]
pub(crate) fn numeric_list_field_borrows_retain_owner_storage() {
    for source in [
        "10:{->4<int32[2]>:=[7,8]};r:&(10.4[1]);x:*r",
        "10:{->4<int32[2]>:=[7,8]};r:&(10.4[1]);x:*r;10.4[1]=9",
        "10:{->4<int32[2]>:=[7,8]};r:&!(10.4[1]);*r=9",
    ] {
        crate::compile(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
    }
    rejects(
        "10:{->4<int32[2]>:=[7,8]};r:&(10.4[1]);10.4[1]=9;x:*r",
        "E302",
    );
    rejects(
        "10:{->4<int32[2]>:=[7,8]};r:&!(10.4[1]);x:10.4[1];y:*r",
        "E302",
    );
}

#[test]
pub(crate) fn numeric_list_sources_use_captured_scopes() {
    let tree = crate::parser::parse("10:{->4<uint8>:2};10.4").unwrap();
    let mut checker = Checker::new();
    checker.stmt(&tree.stmts[0]).unwrap();
    let StmtKind::Expr(value) = &tree.stmts[1].kind else {
        panic!("numeric field")
    };
    let scopes = std::mem::take(&mut checker.scopes);
    let (ty, work) = Checker::list_source(&scopes, value);
    assert_eq!(
        ty,
        Some(&Type::Int {
            signed: false,
            bits: 8
        })
    );
    assert!(work >= 2);
    assert!(Checker::list_source(&checker.scopes, value).0.is_none());
}

#[test]
pub(crate) fn numeric_list_probes_keep_live_and_emitted_roots() {
    for (source, pure) in [
        ("10.4", true),
        ("10:{->4<uint8>:2};10.4", false),
        ("10:{->4<uint8>:2};-10.4", false),
        ("{->10:{->4:7};->11:10.4}", false),
        ("{->11:10.4;->10:{->4:7}}", false),
        ("{->10:{->4:7};->11:@\"core\".literal(10.4)}", true),
    ] {
        let tree = crate::parser::parse(source).unwrap();
        let mut checker = Checker::new();
        for stmt in &tree.stmts[..tree.stmts.len() - 1] {
            checker.stmt(stmt).unwrap();
        }
        let StmtKind::Expr(value) = &tree.stmts.last().unwrap().kind else {
            panic!("numeric expression")
        };
        assert_eq!(
            checker.list_pure(value, true).unwrap().is_some(),
            pure,
            "{source}"
        );
    }
    for (source, shadowed) in [
        ("10.4", true),
        ("10.4+1", true),
        ("11.4", false),
        ("@\"core\".literal(10.4)", false),
    ] {
        let tree = crate::parser::parse(source).unwrap();
        let StmtKind::Expr(value) = &tree.stmts[0].kind else {
            panic!("numeric expression")
        };
        assert_eq!(
            Checker::new().list_shadowed(value, &["10"]).unwrap(),
            shadowed,
            "{source}"
        );
    }
}
