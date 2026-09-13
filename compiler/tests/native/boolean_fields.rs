use super::Case;

#[test]
pub(crate) fn boolean_fields_keep_mixed_records_eligible_for_integer_reads() {
    Case::new("d:@\"debug\";row:{->part:{->enabled:true;->width<uint8>:4};->other:false};n<uint8>:{part:row.part;copy:part;->copy.width};<T>:{-><int32[n]>};v<T>:[7];d.print(v[1]);|row.part.enabled|d.print(\"enabled\")").runs(b"7\nenabled\n");
}

#[test]
pub(crate) fn boolean_fields_preserve_whole_record_effect_and_mutation_gates() {
    for row in [
        "{->enabled:{d.print(9);->true};->width:4}",
        "{->enabled:true;->width:4;d.print(9)}",
        "{->enabled:=true;->width:4}",
        "{->enabled:get();->width:4}",
    ] {
        let source = format!(
            "d:@\"debug\";get<boolean>:(){{->true}};row:{row};<T>:{{n:row.width;-><int32>}}"
        );
        let output = Case::new(&source).command("run", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E211\""), "{source}: {error}");
    }
}
