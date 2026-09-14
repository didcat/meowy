#[test]
pub(crate) fn block_type_equality_infers_normalized_payloads_without_runtime_storage() {
    for (left, right, same) in [
        ("{-><int32>}", "{-><int32>}", true),
        ("{-><int32>}", "{-><uint32>}", false),
        ("{-><int32><null>}", "{-><null><int32><never>}", true),
        ("{local<Type>:<int32>;->local}", "<int32>", true),
        ("<int32>", "{kind:{-><int32>};->kind}", true),
        ("{n<uint8>:2;-><int32[n]>}", "{-><int32[2]>}", true),
        (
            "{-><{a<int32>;b<boolean>}>}",
            "{-><{b<boolean>;a<int32>}>}",
            true,
        ),
        ("{-><&int32>}", "<&!int32>", false),
    ] {
        for (op, expected) in [("==", same), ("!=", !same)] {
            let source =
                format!("<T>:{{flag:({left}){op}({right});|flag|-><int32>;|!flag|-><boolean>}}");
            let program = crate::compile(&source).unwrap();
            assert!(program.body.stmts.is_empty());
            assert!(program.locals.is_empty());
            assert!(program.functions.is_empty());
            let value = if expected { "7" } else { "true" };
            crate::compile(&format!("{source};v<T>:{value}")).unwrap();
        }
    }
    crate::compile("kind<Type>:<int32>;<T>:{flag:kind==({|true|->7<>;|false|-><Missing>});|flag|->kind};v<T>:7").unwrap();
}

#[test]
pub(crate) fn block_type_equality_keeps_scalar_context_and_result_kind_errors() {
    crate::compile(
        "<T>:{n<uint8>:4;flag:({->2})+2==n;other:({->false})==false;|flag&&other|-><int32>};v<T>:7",
    )
    .unwrap();
    for (expr, code) in [
        ("({-><int32>})==({->true})", "E207"),
        ("({->false})!=({-><int32>})", "E207"),
        ("<int32> == ({->1})", "E207"),
        ("({->1})==<int32>", "E207"),
        ("({-><int32>})==true", "E207"),
        ("1==({-><int32>})", "E207"),
        ("({-><int32>})<({-><int32>})", "E207"),
        ("<int32> < ({-><int32>})", "E222"),
        ("({-><int32>;-><int32>})==<int32>", "E205"),
        ("({|false|-><int32>})==<int32>", "E211"),
        ("({kind:<int32>;->kind})==kind", "E201"),
        ("({->field:<int32>})==<int32>", "B001"),
    ] {
        let source = format!("<T>:{{flag:{expr};-><int32>}}");
        let error = crate::compile(&source).unwrap_err().remove(0);
        assert_eq!(error.code, code, "{expr}: {error:?}");
    }
}

#[test]
pub(crate) fn block_type_equality_reports_type_emissions_in_scalar_contexts() {
    for (body, code) in [
        ("n<boolean>:{-><int32>}", "E207"),
        ("n<uint8>:{kind<Type>:<int32>;->kind}", "E207"),
        ("n<boolean>:{-><int32[1/0]>}", "E107"),
        ("n<uint8>:{-><Missing>}", "E202"),
    ] {
        let error = crate::compile(&format!("<T>:{{{body};-><int32>}}"))
            .unwrap_err()
            .remove(0);
        assert_eq!(error.code, code, "{body}: {error:?}");
    }
}
