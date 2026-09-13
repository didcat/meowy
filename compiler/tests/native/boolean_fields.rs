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

#[test]
pub(crate) fn boolean_fields_feed_predicates_copies_scratch_and_compositions() {
    Case::new("d:@\"debug\";row:{->part:{->enabled:true;->width<uint8>:4};->other:false};part:row.part;flag:part.enabled;copy:{->part};n<uint8>:{settings:copy;ready:settings.enabled==flag;|ready|->settings.width;|!ready|->2};<T>:{-><int32[n]>};v<T>:[7];d.print(v[1])").runs(b"7\n");
    super::file_modules::case(
        "m:@\"./facade.mwy\";ready:m.row.enabled;n:{|ready|->4;|!ready|->2};<T>:{-><int32[n]>};v<T>:[7];d:@\"debug\";d.print(v[1])",
        &[("data.mwy", "->row:{->enabled:true;->width:4}"), ("facade.mwy", "m:@\"./data.mwy\";->m")],
    ).runs(b"7\n");
}

#[test]
pub(crate) fn boolean_fields_keep_ancestor_errors_and_effects_in_predicate_reads() {
    let source = "#é#\n|false|{row<{enabled<boolean>;bad<uint8>}>:{->enabled:true;->bad<uint8>:255+1};flag:row.enabled;n<uint8>:{|flag|->4};<T>:{v:n;-><int32>}}";
    let error = meowy::compile(source).unwrap_err().remove(0);
    assert_eq!(error.code, "E107", "{error:?}");
    assert_eq!(error.span.start, source.find("255+1").unwrap());
    for row in [
        "{->part:{->enabled:true};->bad:{->n:1;d.print(9)}}",
        "{->part:{->enabled:true};d.print(9)}",
        "{->part:{->enabled:=true}}",
    ] {
        let source = format!(
            "d:@\"debug\";row:{row};part:row.part;n:{{flag:part.enabled;|flag|->4;|!flag|->2}};<T>:{{v:n;-><int32>}}"
        );
        let output = Case::new(&source).command("run", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E211\""), "{source}: {error}");
    }
}

#[test]
pub(crate) fn boolean_fields_keep_integer_and_direct_module_input_gates() {
    for (source, code) in [
        ("row:{->enabled:true};n<uint8>:{->row.enabled}", "E207"),
        ("row:{->enabled:true};<T>:{v:row.enabled;-><int32>}", "B001"),
        ("row:{->enabled:true};f<boolean>:(){->row.enabled}", "B001"),
    ] {
        assert_eq!(
            meowy::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
    let output = super::file_modules::case(
        "m:@\"./data.mwy\";n:{flag:m.enabled;|flag|->4;|!flag|->2};<T>:{v:n;-><int32>}",
        &[("data.mwy", "->enabled:true")],
    )
    .command("check", &["--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E211\""));
}
