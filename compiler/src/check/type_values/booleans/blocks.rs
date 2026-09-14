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
        ("flag:({->true})==true", "E222"),
        ("flag:({->true})==({->false})", "E207"),
    ] {
        let source = format!("<T>:{{{body};-><int32>}}");
        let error = crate::compile(&source).unwrap_err().remove(0);
        assert_eq!(error.code, code, "{body}: {error:?}");
    }
}
