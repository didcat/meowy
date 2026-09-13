use super::file_modules::case;

#[test]
pub(crate) fn conditional_types_select_scalars_records_and_nested_lists() {
    for (condition, value, output) in [
        ("true", "7", "7\n"),
        ("false", "\"selected\"", "selected\n"),
    ] {
        case(&format!("<T>:{{flag:{condition};|flag|-><int32>;|!flag|-><string>}};v<T>:{value};d:@\"debug\";d.print(v)"), &[]).runs(output.as_bytes());
    }
    case(
        "enabled:true;<Row>:{|enabled|->{<Number>:<int32>;-><{n<Number>}>};|!enabled|-><string>};<Items>:{size<uint8>:4;|size>=4|-><Row[size]>;|size<4|-><Row[2]>};v<Items>:[{->n:3},{->n:7}];d:@\"debug\";d.print(v[2].n)",
        &[],
    ).runs(b"7\n");
}

#[test]
pub(crate) fn conditional_types_keep_function_required_reads_and_runtime_capture_gates() {
    case(
        "m:@\"./data.mwy\";f<int32>:(){<T>:{|m.width>=4|-><int32>;|m.width<4|-><string>};v<T>:7;->v};d:@\"debug\";d.print(f())",
        &[("data.mwy", "->width<uint8>:4")],
    ).runs(b"7\n");
    let output = case(
        "m:@\"./data.mwy\";f<uint8>:(){<T>:{|m.width>=4|-><int32>};->m.width}",
        &[("data.mwy", "->width<uint8>:4")],
    )
    .command("check", &["--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"B001\""));
}
