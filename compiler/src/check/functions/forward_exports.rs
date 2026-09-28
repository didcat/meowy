use super::*;

#[test]
pub(crate) fn forward_exports_reuse_reserved_ids_and_keep_private_definitions() {
    let ast = crate::parser::parse("f<()->int32>;g<()->int32>;private<()->int32>;->g:(){inner<int32>:(){->2};->inner()};private<int32>:(){->g()};->f<int32>:(){->private()};->alias<()->int32>:f").unwrap();
    let mut checker = Checker::new();
    checker.block_start(&ast, None, None, false).unwrap();
    let (point, group) = checker.forward_point(&ast.stmts, 0).unwrap();
    assert_eq!(group.functions, [0, 1, 2]);
    assert_eq!(group.end, 6);
    assert_eq!(checker.functions.len(), 4);
    for (name, id) in [("f", 0), ("g", 1)] {
        assert!(
            matches!(checker.module.values[name], Value::Function { id: found, .. } if found == id)
        );
        assert_eq!(checker.functions[id].as_ref().unwrap().name, name);
    }
    assert!(!checker.module.values.contains_key("private"));
    assert!(!checker.module.values.contains_key("inner"));
    checker
        .forward_group_endpoint(point, &group.functions, ast.span)
        .unwrap();
    checker.stmt(&ast.stmts[group.end]).unwrap();
    assert!(matches!(
        checker.module.values["alias"],
        Value::Function { id: 0, .. }
    ));
    assert_eq!(checker.functions.len(), 4);
    assert_eq!(checker.owner, 0);
}

#[test]
pub(crate) fn forward_exports_publish_nothing_from_a_failed_group() {
    for (tail, code) in [
        ("", "E221"),
        ("g<boolean>:(){->true}", "E221"),
        ("->g<int32>:(){->false}", "E207"),
        ("->g<int32>:(){missing}", "E201"),
        ("->f<int32>:(){->2}", "E221"),
        ("->g<int32>:=(){->2}", "E221"),
        ("'out->g<int32>:(){->2}", "E221"),
    ] {
        let ast = crate::parser::parse(&format!(
            "->old<int32>:(){{->0}};f<()->int32>;g<()->int32>;->f<int32>:(){{->1}};{tail}"
        ))
        .unwrap();
        let mut checker = Checker::new();
        checker.block_start(&ast, None, None, false).unwrap();
        checker.stmt(&ast.stmts[0]).unwrap();
        let error = checker.forward_point(&ast.stmts, 1).unwrap_err();
        assert_eq!(error.code, code, "{tail}");
        assert_eq!(checker.module.values.len(), 1);
        assert!(checker.module.values.contains_key("old"));
        assert!(checker.site.is_none());
        assert!(checker.statement.is_empty());
        let point = checker
            .points
            .iter()
            .find(|point| point.span == ast.stmts[1].span)
            .unwrap();
        assert!(!point.complete);
    }
}

#[test]
pub(crate) fn forward_exports_preserve_signatures_scope_and_collisions() {
    for (source, code) in [
        ("f<(int32)->int32>;->f<int32>:(n<boolean>){->1}", "E221"),
        ("f<()->int32>;->f<boolean>:(){->true}", "E221"),
        ("f<()->int32>;->other<int32>:(){->1}", "E221"),
        ("f<()->int32>;->f<int32>:(){->1};->f<int32>:(){->2}", "E205"),
        ("->f:1;f<()->int32>;->f<int32>:(){->2}", "E203"),
        ("{f<()->int32>;->f<int32>:(){->1}}", "B001"),
        ("outer:(){f<()->int32>;->f<int32>:(){->1}}", "B001"),
        ("x:1;f<()->int32>;->f<int32>:(){->x}", "E221"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
    let ast = crate::parser::parse("f<()->int32>;->f<int32>:(){->1}").unwrap();
    let mut checker = Checker::new();
    checker.block_start(&ast, None, None, false).unwrap();
    checker.scopes.push(Scope::default());
    let error = checker.forward(&ast.stmts, 0).unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("conditional function exports"));
    assert!(checker.module.values.is_empty());
}

#[test]
pub(crate) fn forward_exports_keep_documentation_signatures_and_link_checks() {
    let source = "f<()->int32>;#| Call [[f]]. |#->f:(){->1}";
    let parsed = crate::parser::parse_documented(source).unwrap();
    let model = crate::documentation::Model::at(source, &parsed, 0, false).unwrap();
    let (_, model) = crate::check::check_documented(&parsed.block, Some(model)).unwrap();
    let model = model.unwrap();
    let entry = model
        .entries
        .iter()
        .find(|entry| entry.name == "f" && entry.kind == crate::documentation::Kind::Function)
        .unwrap();
    assert!(entry.public);
    assert!(entry.checked);
    assert_eq!(entry.signature, "()->int32");
    assert_eq!(
        entry.links[0].resolved,
        model.starts.get(&entry.span.start).copied()
    );
    for source in [
        "f<()->int32>;#| [[missing]] |#->f:(){->1}",
        "private<int32>:(){->1};f<()->int32>;#| [[private]] |#->f:(){->1}",
    ] {
        let parsed = crate::parser::parse_documented(source).unwrap();
        let model = crate::documentation::Model::at(source, &parsed, 0, false).unwrap();
        assert_eq!(
            crate::check::check_documented(&parsed.block, Some(model)).unwrap_err()[0].code,
            "E802"
        );
    }
}

#[test]
pub(crate) fn forward_exports_keep_publication_atomic_at_the_work_limit() {
    let ast = crate::parser::parse("f<()->int32>;g<()->int32>;->f:(){->1};->g:(){->2}").unwrap();
    let mut checker = Checker::new();
    checker.block_start(&ast, None, None, false).unwrap();
    let start = checker.flow.work;
    checker.forward(&ast.stmts, 0).unwrap();
    let work = checker.flow.work - start;
    for spare in [0, 1] {
        let mut checker = Checker::new();
        checker.block_start(&ast, None, None, false).unwrap();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.forward(&ast.stmts, 0);
        if spare == 0 {
            result.unwrap();
            assert_eq!(checker.module.values.len(), 2);
        } else {
            assert_eq!(result.unwrap_err().code, "B001");
            assert!(checker.module.values.is_empty());
        }
    }
}
