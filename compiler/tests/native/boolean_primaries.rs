use super::file_modules::case;

#[test]
pub(crate) fn boolean_primaries_keep_values_aliases_projections_and_reexports() {
    for value in [false, true] {
        for fields in ["", ";->label:\"ready\""] {
            let data = format!("private:{value};->{{->private;unused:7}}{fields}");
            case(
                "s:@\"./data.mwy\";alias:s;copy:!!alias;m:@\"./outer.mwy\";n:{flag:m&&true;ok:flag==copy&&m.enabled==s;|ok|->4;|!ok|->2};<T>:{-><int32[n]>};v<T>:[7];f<int32>:(){<U>:{count:n;-><int32[count]>};u<U>:[9];->u[1]};d:@\"debug\";d.print(v[1]);d.print(n);d.print(!m);d.print(f())",
                &[
                    ("data.mwy", &data),
                    ("facade.mwy", "m:@\"./data.mwy\";->m;->enabled<boolean>:m"),
                    ("outer.mwy", "m:@\"./facade.mwy\";->m"),
                ],
            ).runs(format!("7\n4\n{}\n9\n", !value).as_bytes());
            case(
                "m:@\"./data.mwy\";n:{flag:!m;|flag|->2;|!flag|->4};<T>:{-><int32[n]>};d:@\"debug\";d.print(n)",
                &[("data.mwy", &data)],
            ).runs(format!("{}\n", if value { 4 } else { 2 }).as_bytes());
        }
    }
    case(
        "m:@\"./data.mwy\";alias:m;<Row>:{->alias<>};row<Row>:{->false;->label:\"copy\"};n:{m:@\"./data.mwy\";flag:m&&true;|flag|->4;|!flag|->2};<T>:{-><int32[n]>};d:@\"debug\";d.print(alias.label);d.print(row.label);d.print(n)",
        &[("data.mwy", "->true;->label:\"ready\"")],
    ).runs(b"ready\ncopy\n4\n");
}

#[test]
pub(crate) fn boolean_primaries_reject_unproved_initializers_through_facades() {
    for data in [
        "d:@\"debug\";->{->true;d.print(\"must not run\")}",
        "value:=true;->value",
        "|true|->true",
        "get<boolean>:(){->true};->get()",
    ] {
        for facade in ["m:@\"./data.mwy\";->m", "m:@\"./data.mwy\";copy:!m;->!copy"] {
            let case = case(
                "m:@\"./facade.mwy\";n:{flag:!m;->4};<T>:{v:n;-><int32>}",
                &[("data.mwy", data), ("facade.mwy", facade)],
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
pub(crate) fn boolean_primaries_keep_kind_scope_and_record_boundaries() {
    for (body, data, code) in [
        ("copy<boolean>:m", "->true", "B001"),
        ("<T>:{flag<boolean>:m;-><int32>}", "->true", "B001"),
        ("<T>:{flag:m&&true;-><int32>}", "->true", "B001"),
        ("<T>:{-><int32[m]>}", "->true", "B001"),
        ("f<boolean>:(){->!m}", "->true", "B001"),
        ("copy:!!m;f<boolean>:(){->copy}", "->true", "B001"),
        (
            "f<int32>:(){n:{flag:!m;->4};<T>:{v:n;-><int32>};->1}",
            "->true",
            "B001",
        ),
        (
            "n:{copy:{->m};->4};<T>:{v:n;-><int32>}",
            "->true;->enabled:true",
            "E211",
        ),
        (
            "n:{flag:m.private;->4};<T>:{v:n;-><int32>}",
            "private:true;->private",
            "E201",
        ),
        (
            "row:{->true;->n:4};n:{flag:!row;->4};<T>:{v:n;-><int32>}",
            "->true",
            "E211",
        ),
    ] {
        let source = format!("m:@\"./data.mwy\";{body}");
        let output = case(&source, &[("data.mwy", data)]).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{body}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(&format!("\"code\":\"{code}\"")),
            "{body}: {error}"
        );
    }
}
