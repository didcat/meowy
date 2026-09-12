use super::Case;

#[test]
pub(crate) fn integer_blocks_select_exact_width_values_and_restore_branch_scope() {
    for (pick, count) in [("true", 4), ("false", 2)] {
        let source = format!(
            "d:@\"debug\";pick:{pick};n<uint8>:{{base<uint8>:4;|pick|->base;|!pick|->2;unused:base+1}};alias:n;<T>:{{-><int32[alias]>}};v<T>:[7];d.print(v[1]);d.print(alias)"
        );
        Case::new(&source).runs(format!("7\n{count}\n").as_bytes());
    }
    Case::new("d:@\"debug\";n<uint8>:{base<uint8>:4;|true|base<uint8>:200;->base};<T>:{-><int32[n]>};v<T>:[7];d.print(v[1]);d.print(n)").runs(b"7\n4\n");
    Case::new("d:@\"debug\";n<uint64>:{|true|->18446744073709551615;|false|->0};<T>:{-><int32[n&3]>};v<T>:[7];d.print(v[1]);d.print(n)").runs(b"7\n18446744073709551615\n");
    for (source, code) in [
        (
            "n<uint8>:{|true|->255;|false|->0};<T>:{v:n+1;-><int32>}",
            "E107",
        ),
        ("wide<uint16>:4;n<uint8>:{|true|->wide;|false|->2}", "E207"),
        ("n<uint8>:{|true|->4;|true|->2}", "E205"),
        ("n<uint8>:{|false|->4}", "E204"),
        ("n:{|true|temp:4;->temp}", "E201"),
    ] {
        assert_eq!(
            meowy::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn integer_blocks_skip_only_unselected_effects_and_keep_integer_scratch() {
    for block in [
        "{|false|d.print(9);->4}",
        "{->4;|false|bad<uint8>:255+1}",
        "{|false|unused:true;->4}",
        "{|true||get()|->4}",
        "{|true| |!false|->4}",
    ] {
        let source = format!(
            "d:@\"debug\";get<boolean>:(){{d.print(9);->true}};n:{block};<T>:{{-><int32[n]>}};v<T>:[7];d.print(v[1])"
        );
        Case::new(&source).runs(b"7\n");
    }
    for block in [
        "{|true|d.print(9);->4}",
        "{->4;|true|d.print(9)}",
        "{|true|unused:=1;->4}",
        "{|true|unused:true;->4}",
        "{|get()|unused:1;->4}",
        "{|true|{unused:1};->4}",
    ] {
        let source = format!(
            "d:@\"debug\";get<boolean>:(){{d.print(9);->true}};n:{block};<T>:{{v:n;-><int32>}}"
        );
        let output = Case::new(&source).command("run", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E211\""), "{source}: {error}");
    }
}

#[test]
pub(crate) fn integer_blocks_retain_selected_predicate_primary_and_tail_error_spans() {
    for block in [
        "{|base+1==0|->4}",
        "{|true|bad<uint8>:255+1;->4}",
        "{->4;|true|bad<uint8>:255+1}",
        "{|true| |true|->base+1}",
    ] {
        let source = format!(
            "|false|{{base<uint8>:255;n<uint8>:{block};alias:n;<T>:{{v:alias;-><int32>}}}}"
        );
        let error = meowy::compile(&source).unwrap_err().remove(0);
        assert_eq!(error.code, "E107", "{source}: {error:?}");
        let start = source
            .find("base+1")
            .or_else(|| source.find("255+1"))
            .unwrap();
        assert_eq!(error.span.start, start);
    }
}
