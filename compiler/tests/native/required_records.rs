use super::file_modules::case;

#[test]
pub(crate) fn required_records_support_aliases_fields_and_scoped_function_reads() {
    case(
        "m:@\"./facade.mwy\";<Items>:{record:m.row;part:record.part;alias:part;flag:record.enabled;count<uint8>:{copy:alias;->copy.width};|flag|-><int32[count]>;|!flag|-><string>};v<Items>:[3,7];f<int32>:(){p:@\"./facade.mwy\";<T>:{r:p.row;-><int32[r.part.width]>};v<T>:[9];->v[1]};d:@\"debug\";d.print(v[2]);d.print(f())",
        &[("data.mwy", "private<uint8>:4;->row:{->enabled:true;->part:{->width:private}}"), ("facade.mwy", "m:@\"./data.mwy\";->m")],
    ).runs(b"7\n9\n");
}

#[test]
pub(crate) fn required_records_keep_privacy_capture_and_namespace_gates() {
    for (body, code) in [
        ("<T>:{copy:m;-><int32>}", "E211"),
        ("<T>:{copy:m.private;-><int32>}", "E201"),
        (
            "<T>:{copy:m.row;count:copy.enabled;-><int32[count]>}",
            "B001",
        ),
        (
            "f<int32>:(){<T>:{copy:m.row;-><int32[copy.width]>};->m.row.width}",
            "B001",
        ),
        (
            "f<int32>:(row<{width<uint8>}>){<T>:{copy:row;-><int32>};->1}",
            "E211",
        ),
    ] {
        let output = case(
            &format!("m:@\"./data.mwy\";{body}"),
            &[(
                "data.mwy",
                "private:{->width:4};->row:{->enabled:true;->width<uint8>:4}",
            )],
        )
        .command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{body}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(&format!("\"code\":\"{code}\"")),
            "{body}: {error}"
        );
    }
}
