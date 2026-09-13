use super::file_modules::case;

#[test]
pub(crate) fn boolean_modules_preserve_values_aliases_and_named_reexports() {
    for enabled in [false, true] {
        let data = format!("private:{enabled};->enabled:private;->other:!private");
        case(
            "m:@\"./outer.mwy\";alias:m.enabled;n:{ok:alias!=m.other;ready:{->ok&&m.copy};|ready|->4;|!ready|->2};<T>:{-><int32[n]>};v<T>:[7];d:@\"debug\";d.print(v[1]);d.print(n)",
            &[("data.mwy", &data), ("facade.mwy", "m:@\"./data.mwy\";->m;->copy:m.enabled"), ("outer.mwy", "m:@\"./facade.mwy\";->m")],
        ).runs(format!("7\n{}\n", if enabled { 4 } else { 2 }).as_bytes());
    }
}

#[test]
pub(crate) fn boolean_modules_preserve_initializer_and_module_boundaries() {
    for (data, code) in [
        ("d:@\"debug\";->enabled:{->true;d.print(9)}", "E211"),
        ("get<boolean>:(){->true};->enabled:get()", "E211"),
        ("value:=true;->enabled:value", "E211"),
        ("|true|->enabled:true", "E211"),
        ("->enabled:=true", "B001"),
    ] {
        let output = case(
            "m:@\"./data.mwy\";n:{unused:m.enabled;->4};<T>:{v:n;-><int32>}",
            &[("data.mwy", data)],
        )
        .command("run", &["--json"]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(&format!("\"code\":\"{code}\"")),
            "{data}: {error}"
        );
    }
    for (source, data, code) in [
        (
            "m:@\"./data.mwy\";flag:m.private",
            "private:true;->enabled:private",
            "E201",
        ),
        (
            "m:@\"./data.mwy\";f<boolean>:(){->m.enabled}",
            "->enabled:true",
            "B001",
        ),
        (
            "m:@\"./data.mwy\";n:{unused:!m;->4};<T>:{v:n;-><int32>}",
            "->true",
            "E211",
        ),
        (
            "m:@\"./data.mwy\";n:{copy:{->m};->4};<T>:{v:n;-><int32>}",
            "->enabled:true",
            "E211",
        ),
        (
            "m:@\"./data.mwy\";<T>:{flag:m.enabled;-><int32>}",
            "->enabled:true",
            "B001",
        ),
    ] {
        let output = case(source, &[("data.mwy", data)]).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(&format!("\"code\":\"{code}\"")),
            "{source}: {error}"
        );
    }
}
