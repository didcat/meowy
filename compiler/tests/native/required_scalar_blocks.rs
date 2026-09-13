use super::file_modules::case;

#[test]
pub(crate) fn required_scalar_blocks_construct_lists_with_scoped_matchers() {
    case(
        "<Items>:{ready<boolean>:{flag:false;->!flag};count<uint8>:{base<uint8>:{->2};|ready|->{->base*2};|!ready|->2;unused:7};-><int32[count]>};v<Items>:[3,7];d:@\"debug\";d.print(v[2])",
        &[],
    ).runs(b"7\n");
    case(
        "<T>:{<Byte>:<uint8>;n<Byte>:{inner:{|true|-><int32>};copy<Byte>:{->4};->copy};flag<boolean>:{|n>=4|->false;|n<4|->true};|flag|-><string>;|!flag|-><int32[n]>};v<T>:[9];d:@\"debug\";d.print(v[1])",
        &[],
    ).runs(b"9\n");
}

#[test]
pub(crate) fn required_scalar_blocks_read_module_inputs_without_runtime_captures() {
    case(
        "m:@\"./data.mwy\";f<int32>:(){<T>:{count<uint8>:{->m.width};flag<boolean>:{->m.enabled};|flag|-><int32[count]>;|!flag|-><string>};v<T>:[7];->v[1]};d:@\"debug\";d.print(f())",
        &[("data.mwy", "->width<uint8>:4;->enabled:true")],
    ).runs(b"7\n");
    for source in [
        "f<int32>:(v<uint8>){<T>:{n<uint8>:{->v};-><int32[n]>};->1}",
        "mutable:=true;<T>:{n<boolean>:{->mutable};->n<>}",
    ] {
        let output = case(source, &[]).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E211\""));
    }
}
