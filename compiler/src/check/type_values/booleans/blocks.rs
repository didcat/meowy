#[test]
pub(crate) fn logical_blocks_preserve_boolean_values_without_runtime_storage() {
    for a in [false, true] {
        for b in [false, true] {
            for (expr, expected) in [
                (format!("({{->{a}}})&&({{->{b}}})"), a && b),
                (format!("({{->{a}}})||({{->{b}}})"), a || b),
                (format!("!({{->{a}}})"), !a),
            ] {
                let values = if expected { "1,2,3,4" } else { "1,2" };
                let source = format!("<T>:{{flag:{expr};|flag|-><int32[4]>;|!flag|-><int32[2]>}}");
                let program = crate::compile(&source).unwrap();
                assert!(program.locals.is_empty());
                assert!(program.body.stmts.is_empty());
                crate::compile(&format!("{source};v<T>:[{values}]")).unwrap();
            }
        }
    }
    crate::compile("<T>:{r:{->flag:({local:false;->!({->local})})&&({->true})};|({->r.flag})|-><int32>};v<T>:7").unwrap();
    crate::compile("<T>:{true:false;flag:!({->true});|flag|-><int32>};v<T>:7").unwrap();
}

#[test]
pub(crate) fn logical_blocks_skip_initializers_and_preserve_kind_and_scope_gates() {
    for source in [
        "<T>:{flag:false&&({local<Missing>:unknown();->local});-><int32>}",
        "<T>:{flag:true||({->1/0});-><int32>}",
    ] {
        crate::compile(source).unwrap();
    }
    for (body, code) in [
        ("flag:true&&({->1})", "E207"),
        ("flag:false||({->1/0})", "E107"),
        ("flag:!({})", "E204"),
        ("flag:!({->false;->true})", "E205"),
        ("flag:!({->false;tail:1/0})", "E107"),
        ("flag:({local:true;->local})&&local", "E201"),
        ("flag:false&&({local:=true;->local})", "B001"),
        ("flag:true||({->field:true})", "B001"),
        ("flag:!({-><boolean>})", "B001"),
    ] {
        let source = format!("<T>:{{{body};-><int32>}}");
        let error = crate::compile(&source).unwrap_err().remove(0);
        assert_eq!(error.code, code, "{body}: {error:?}");
    }
}

#[test]
pub(crate) fn logical_blocks_charge_selected_work_once_and_restore_failed_scopes() {
    use crate::ast::StmtKind;
    use crate::check::{
        Checker, Constant, Value,
        type_values::{MAX_WORK, Work},
    };
    for (source, expected, cost) in [
        ("false&&({local<Missing>:unknown();->local})", false, 2),
        ("true||({->1/0})", true, 2),
        ("({->false})&&({->missing()})", false, 6),
        ("({->true})&&({->false})", false, 11),
        ("!({->false})", true, 6),
    ] {
        let parsed = crate::parser::parse(&format!("value:{source}")).unwrap();
        let StmtKind::Bind { value, .. } = &parsed.stmts[0].kind else {
            panic!("binding")
        };
        let mut checker = Checker::new();
        let scopes = checker.scopes.len();
        checker.type_work = Some(Work::default());
        assert!(
            matches!(checker.type_boolean(value,None).unwrap(),Value::Static {value:Constant::Bool(found),..} if found==expected)
        );
        assert_eq!(checker.type_work.as_ref().unwrap().visits, cost, "{source}");
        assert_eq!(checker.type_work.as_ref().unwrap().nodes, 0);
        for (visits, accepted) in [(MAX_WORK - cost, true), (MAX_WORK - cost + 1, false)] {
            checker.type_work = Some(Work {
                visits,
                ..Work::default()
            });
            let result = checker.type_boolean(value, None);
            if accepted {
                assert!(result.is_ok(), "{:?}", result.err());
            } else {
                assert_eq!(result.err().unwrap().code, "B001");
            }
            assert_eq!(checker.scopes.len(), scopes);
            assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
            assert!(!checker.required);
        }
        assert!(checker.locals.is_empty());
    }
}

#[test]
pub(crate) fn logical_blocks_bound_nested_execution_and_preserve_checked_metadata() {
    use crate::ast::{Block, Expr, ExprKind, Span, Stmt, StmtKind};
    use crate::check::{Checker, type_values::Work};
    for depth in [62, 63] {
        let span = Span::new(0, 1);
        let mut expr = Expr {
            span,
            kind: ExprKind::Name("true".into()),
        };
        for _ in 0..depth {
            expr = Expr {
                span,
                kind: ExprKind::Block(Block {
                    span,
                    label: None,
                    stmts: vec![Stmt {
                        span,
                        kind: StmtKind::Emit {
                            label: None,
                            name: None,
                            ty: None,
                            mutable: false,
                            value: expr,
                        },
                    }],
                }),
            };
        }
        let mut checker = Checker::new();
        checker.type_work = Some(Work::default());
        let scopes = checker.scopes.len();
        let result = checker.type_boolean(&expr, None);
        if depth == 62 {
            assert!(result.is_ok(), "{:?}", result.err());
        } else {
            assert_eq!(result.err().unwrap().code, "B001");
        }
        assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
        assert_eq!(checker.scopes.len(), scopes);
    }
    let source = "#| Items. |#<T>:{#| Ready. |#flag:!({#| Local. |#local:false;->local})&&({->true});|flag|-><int32[4]>;|!flag|-><string>}";
    let (_, model) = crate::documentation::checked(source, true).unwrap();
    let model = model.unwrap();
    for (name, signature) in [("flag", "boolean"), ("local", "boolean"), ("T", "int32[4]")] {
        let entry = model
            .entries
            .iter()
            .find(|entry| entry.name == name)
            .unwrap();
        assert!(entry.checked);
        assert_eq!(entry.signature, signature);
    }
    let source = "<T>:{flag:false&&({#| Skipped. |#local:true;->local});-><int32>}";
    assert_eq!(
        crate::documentation::checked(source, true).unwrap_err()[0].code,
        "B001"
    );
}
