use super::file_modules::case;

#[test]
pub(crate) fn required_booleans_read_fields_exports_and_scoped_imports() {
    for value in [false, true] {
        let data = format!(
            "private:{value};->private;->flag:private;->row:{{->part:{{->flag:private}}}};->width<uint8>:4;->label:\"ready\""
        );
        case(
            "m:@\"./facade.mwy\";alias:m;local:{->flag:false};<T>:{a:alias.flag;b:alias.flag;c:alias.row.part.flag;d:local.flag;copy<boolean>:(c);n<uint8>:m.width;-><(copy<>)[n]>};v<T>:[false,true];f<boolean>:(){<U>:{a:m.flag;b:m.flag;->a<>};v<U>:true;->v};g<boolean>:(){p:@\"./primary.mwy\";<U>:{flag:p.flag;copy:flag;->copy<>};v<U>:false;->v};d:@\"debug\";d.print(v[2]);d.print(f());d.print(g());d.print(alias.label)",
            &[
                ("data.mwy", &data),
                ("facade.mwy", "m:@\"./data.mwy\";->m"),
                ("primary.mwy", "m:@\"./data.mwy\";->flag:!!m"),
            ],
        ).runs(b"true\ntrue\nfalse\nready\n");
    }
}

#[test]
pub(crate) fn required_booleans_reject_unproved_field_inputs() {
    for data in [
        "d:@\"debug\";value:{->true;d.print(9)};->value;->flag:value;->row:{->flag:value}",
        "value:=true;->value;->flag:value;->row:{->flag:value}",
        "get<boolean>:(){->true};value:get();->value;->flag:value;->row:{->flag:value}",
        "|true|->true;|true|->flag:true;|true|->row:{->flag:true}",
    ] {
        for binding in ["flag:m.flag", "flag:m.row.flag"] {
            let source = format!("m:@\"./facade.mwy\";<T>:{{{binding};-><int32>}}");
            let case = case(
                &source,
                &[("data.mwy", data), ("facade.mwy", "m:@\"./data.mwy\";->m")],
            );
            for profile in ["debug", "release"] {
                let output = case.command("run", &["--profile", profile, "--json"]);
                assert_eq!(output.status.code(), Some(1));
                assert!(output.stdout.is_empty());
                let error = String::from_utf8_lossy(&output.stderr);
                assert!(error.contains("\"code\":\"E211\""), "{data}: {error}");
            }
        }
    }
}

#[test]
pub(crate) fn required_booleans_preserve_kind_privacy_identity_and_capture_gates() {
    for (body, code) in [
        ("<T>:{flag<int32>:m.flag;-><int32>}", "E207"),
        ("<T>:{flag<boolean>:m.width;-><int32>}", "E207"),
        ("<T>:{flag:m.flag;-><int32[flag]>}", "B001"),
        ("<T>:{-><int32[m.flag]>}", "B001"),
        ("<T>:{flag:m.private;-><int32>}", "E201"),
        ("<T>:{flag:m;-><int32>}", "E211"),
        ("<T>:{flag:m.flag;->flag}", "E211"),
        ("f<boolean>:(){<T>:{flag:m.flag;->flag<>};->m.flag}", "B001"),
        ("f<boolean>:(){<T>:{flag:m.flag;->flag<>};->!m}", "B001"),
        ("<T>:{flag:m.flag;other:!flag;-><int32>}", "B001"),
    ] {
        let source = format!("m:@\"./data.mwy\";{body}");
        let output = case(
            &source,
            &[(
                "data.mwy",
                "private:true;->private;->flag:private;->width<uint8>:4",
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
