use super::Case;

#[test]
pub(crate) fn boolean_blocks_keep_nested_bindings_aliases_and_tail_work() {
    for block in [
        "{->true}",
        "{base<uint8>:2;test:{step:base+2;->step==4};->test;unused:base+1}",
        "{flag:true;alias:!flag;->!alias;unused:{->false}}",
    ] {
        let source = format!(
            "d:@\"debug\";flag:{block};alias:flag;row:{{|alias|->n:4;|!alias|->n:2}};<T>:{{-><int32[row.n]>}};v<T>:[7];d.print(v[1]);d.print(row.n)"
        );
        Case::new(&source).runs(b"7\n4\n");
    }
    Case::new("d:@\"debug\";row:{flag:{n:4;->n==4};|flag|->width:4;|!flag|->width:2};<T>:{-><int32[row.width]>};v<T>:[7];d.print(v[1])").runs(b"7\n");
}

#[test]
pub(crate) fn boolean_blocks_retain_first_failures_before_and_after_the_primary() {
    for block in [
        "{bad<uint8>:255+1;->true}",
        "{->true;bad<uint8>:255+1}",
        "{flag<boolean>:{->true;bad<uint8>:255+1};->flag}",
        "{bad<uint8>:255+1;unused:get();->true}",
    ] {
        let source = format!(
            "get<boolean>:(){{->true}};|false|{{flag<boolean>:{block};alias:!flag;row<{{part<{{n<uint8>}}>}}>:{{|alias|->part:{{->n<uint8>:4}}}};copy:row.part;<T>:{{n:copy.n;-><int32>}}}}"
        );
        let error = meowy::compile(&source).unwrap_err().remove(0);
        assert_eq!(error.code, "E107", "{source}: {error:?}");
        assert_eq!(error.span.start, source.find("255+1").unwrap());
    }
}

#[test]
pub(crate) fn boolean_blocks_keep_effects_mutation_branches_and_record_scratch_gated() {
    for block in [
        "{d.print(1);->true}",
        "{->true;d.print(1)}",
        "{unused:=1;->true}",
        "{->true;unused:\"x\"}",
        "{unused:{->n:4};->true}",
        "{|true|->true}",
        "{->get()}",
        "{->true;unused:get()}",
    ] {
        let source = format!(
            "d:@\"debug\";get<boolean>:(){{->true}};flag:{block};row:{{|flag|unused:1;->n:4}};<T>:{{n:row.n;-><int32>}}"
        );
        let output = Case::new(&source).command("run", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E211\""), "{source}: {error}");
    }
}

#[test]
pub(crate) fn boolean_blocks_preserve_integer_block_and_required_scratch_boundaries() {
    for (source, code) in [
        ("n:{flag:true;->4};<T>:{n:n;-><int32>}", "E211"),
        ("flag:{->true};<T>:{flag:flag;-><int32>}", "B001"),
    ] {
        assert_eq!(
            meowy::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
    Case::new("d:@\"debug\";flag:{->true;d.print(\"tail\")};|flag|d.print(\"body\")")
        .runs(b"tail\nbody\n");
}
