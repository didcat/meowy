use super::*;

pub(super) fn export(checker: &mut Checker, stmt: &ast::Stmt) -> Result<Option<hir::FunctionId>> {
    let ast::StmtKind::Emit {
        label,
        name,
        ty,
        mutable,
        value,
    } = &stmt.kind
    else {
        panic!("export statement")
    };
    checker.export_function(
        label.as_deref(),
        name.as_deref(),
        ty.as_ref(),
        *mutable,
        value,
        stmt.span,
    )
}

#[test]
pub(crate) fn function_exports_return_exact_definition_and_reexport_ids() {
    let ast = crate::parser::parse(
        "->outer<int32>:(){inner<int32>:(){->1};->inner()};->later<int32>:(){->2};->alias<()->int32>:outer;->again<()->int32>:alias",
    )
    .unwrap();
    let mut checker = Checker::new();
    checker.block_start(&ast, None, None, false).unwrap();
    for ((stmt, name), id) in ast
        .stmts
        .iter()
        .zip(["outer", "later", "alias", "again"])
        .zip([0, 2, 0, 0])
    {
        assert_eq!(export(&mut checker, stmt).unwrap(), Some(id));
        assert!(
            matches!(checker.module.values[name], Value::Function { id: found, .. } if found == id)
        );
        assert_eq!(checker.functions[id].as_ref().unwrap().id, id);
    }
    assert_eq!(checker.functions.len(), 3);
    assert!(!checker.module.values.contains_key("inner"));
}

#[test]
pub(crate) fn function_exports_keep_unhandled_forms_without_declaring() {
    for source in ["->value:1", "->1", "'out->f<int32>:(){->1}"] {
        let ast = crate::parser::parse(source).unwrap();
        let mut checker = Checker::new();
        checker.block_start(&ast, None, None, false).unwrap();
        assert_eq!(export(&mut checker, &ast.stmts[0]).unwrap(), None);
        assert!(checker.module.values.is_empty());
        assert!(checker.functions.is_empty());
    }
    let ast = crate::parser::parse("->f<int32>:(){->1}").unwrap();
    for nested in [false, true] {
        let mut checker = Checker::new();
        checker.block_start(&ast, None, None, false).unwrap();
        if nested {
            checker.block_start(&ast, None, None, false).unwrap();
        } else {
            checker.owner = 1;
        }
        assert_eq!(export(&mut checker, &ast.stmts[0]).unwrap(), None);
        assert!(checker.module.values.is_empty());
        assert!(checker.functions.is_empty());
    }
}

#[test]
pub(crate) fn function_exports_keep_validation_before_returning_an_identity() {
    for (source, code) in [
        ("->f:(){->1}", "E214"),
        ("->f<int32>:=(){->1}", "B001"),
        ("->f<int32>:(){missing}", "E201"),
        ("->f<int32>:(){->false}", "E207"),
        ("->f<int32>:(){->1};->f<int32>:(){->2}", "E205"),
        ("->f:1;->f<int32>:(){->2}", "E205"),
        ("f:1;->f<int32>:(){->2}", "E203"),
        ("f<int32>:(){->1};->alias:f", "E214"),
        ("f<int32>:(){->1};->alias<()->boolean>:f", "E207"),
        ("|true|->f<int32>:(){->1}", "B001"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
    let ast = crate::parser::parse("->f<int32>:(){->1}").unwrap();
    let mut checker = Checker::new();
    checker.block_start(&ast, None, None, false).unwrap();
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    let error = export(&mut checker, &ast.stmts[0]).unwrap_err();
    assert_eq!(error.code, "B001");
    assert_eq!(
        error.message,
        "module export budget exhausted is not supported by this bootstrap compiler"
    );
    assert!(checker.module.values.is_empty());
    assert!(checker.functions.is_empty());
}
