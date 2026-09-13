use super::file_modules::case;

#[test]
pub(crate) fn required_boolean_logic_reads_modules_in_functions_without_capture() {
    case(
        "m:@\"./facade.mwy\";<T>:{flag:!m||m.flag;copy:flag&&true;->copy<>};v<T>:false;f<boolean>:(){p:@\"./facade.mwy\";<U>:{flag:p&&(!p.flag||true);->flag<>};v<U>:true;->v};d:@\"debug\";d.print(v);d.print(f());d.print(m.label)",
        &[("data.mwy", "->true;->flag:false;->label:\"ready\""), ("facade.mwy", "m:@\"./data.mwy\";->m")],
    ).runs(b"false\ntrue\nready\n");
}

#[test]
pub(crate) fn required_boolean_logic_checks_skipped_names_types_and_purity() {
    for (body, code) in [
        ("<T>:{flag:false&&missing;->flag<>}", "E201"),
        ("<T>:{flag:true||m.private;->flag<>}", "E201"),
        ("<T>:{flag:false&&m.width;->flag<>}", "E222"),
        ("<T>:{flag:!4;->flag<>}", "E222"),
        ("<T>:{flag:true&&m.bad;->flag<>}", "E211"),
        ("<T>:{flag:false||m.bad;->flag<>}", "E211"),
        ("<T>:{flag:false&&get();->flag<>}", "B001"),
        ("f<boolean>:(){<T>:{flag:!m;->flag<>};->!m}", "B001"),
    ] {
        let source = format!("m:@\"./data.mwy\";get<boolean>:(){{->true}};{body}");
        let output = case(
            &source,
            &[(
                "data.mwy",
                "d:@\"debug\";private:true;->true;->width:4;->bad:{d.print(9);->false}",
            )],
        )
        .command("run", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{body}");
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(&format!("\"code\":\"{code}\"")),
            "{body}: {error}"
        );
    }
    case(
        "m:@\"./data.mwy\";f<boolean>:(runtime<boolean>){<T>:{a:false&&runtime;b:true||m.bad;->a<>};v<T>:false;->v};d:@\"debug\";d.print(f(true))",
        &[("data.mwy", "d:@\"debug\";->bad:{d.print(9);->false}")],
    ).runs(b"9\nfalse\n");
}
