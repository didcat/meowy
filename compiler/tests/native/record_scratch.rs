use super::Case;

#[test]
pub(crate) fn record_scratch_preserves_fields_aliases_primaries_and_branch_scope() {
    Case::new("d:@\"debug\";n<uint8>:{row:{->z<uint8>:1;->part:{->n<uint8>:4}};alias:row;part:alias.part;|true|part:{->n<uint8>:2};->part.n;unused:{->n:0}};ok:{row:{->n<uint8>:4};part:row;->part.n==4;unused:{->n:0}};<T>:{-><int32[n]>};v<T>:[1,2,3,7];|ok|d.print(v[4]);d.print(n)").runs(b"7\n4\n");
    Case::new("d:@\"debug\";row:{->part:{->n<uint8>:4};unused:1};n<uint8>:{part:row.part;copy:{->part};->copy.n};<T>:{-><int32[n]>};v<T>:[7];d.print(v[1])").runs(b"7\n");
}

#[test]
pub(crate) fn record_scratch_keeps_ancestor_and_unused_record_failures() {
    for body in [
        "part<{n<uint8>}>:{->n<uint8>:4};row<{part<{n<uint8>}>;bad<uint8>}>:{->part:part;->bad<uint8>:255+1};copy:row.part;->copy.n",
        "->4;row<{n<uint8>}>:{->n<uint8>:1;bad<uint8>:255+1}",
    ] {
        let source = format!("#é#\n|false|{{n<uint8>:{{{body}}};<T>:{{v:n;-><int32>}}}}");
        let error = meowy::compile(&source).unwrap_err().remove(0);
        assert_eq!(error.code, "E107", "{source}: {error:?}");
        assert_eq!(error.span.start, source.find("255+1").unwrap());
    }
    let source = "|false|{ok<boolean>:{row<{n<uint8>}>:{->n<uint8>:4;bad<uint8>:255+1};->row.n==4};n<uint8>:{|ok|->4};<T>:{v:n;-><int32>}}";
    let error = meowy::compile(source).unwrap_err().remove(0);
    assert_eq!(error.code, "E107", "{error:?}");
    assert_eq!(error.span.start, source.find("255+1").unwrap());
}

#[test]
pub(crate) fn record_scratch_requires_complete_eligible_immutable_shapes() {
    for body in [
        "row:{->part:{->n:4};->bad:{->n:1;d.print(9)}};part:row.part;->part.n",
        "row:{->n:4;d.print(9)};->row.n",
        "->4;unused:{->n:1;d.print(9)}",
        "row:={->n:4};->row.n",
        "row:{->n:=4};->row.n",
        "row:{->n:4;->flag:=true};->row.n",
        "row:{->1;->n:4};->row.n",
        "row:{};->4",
        "row:get();->row.n",
    ] {
        let source = format!(
            "d:@\"debug\";get<{{n<int32>}}>:(){{->n:4}};n:{{{body}}};<T>:{{v:n;-><int32>}}"
        );
        let output = Case::new(&source).command("run", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E211\""), "{source}: {error}");
    }
}

#[test]
pub(crate) fn record_scratch_charges_ancestors_projections_and_unused_tails() {
    let tail = (1..12)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let record = format!("{{->part:{{->n<uint8>:4}};v0:1;{tail}}}");
    for (body, heavy) in [
        (format!("row:{record};part:row.part;->part.n"), true),
        (format!("->4;unused:{record}"), true),
        (format!("->4;|false|unused:{record}"), false),
        (
            format!("ok:{{row:{record};part:row.part;->part.n==4}};|ok|->4;|!ok|->2"),
            true,
        ),
    ] {
        let data = format!("n<uint8>:{{{body}}};->width:n");
        let files = [
            ("data.mwy", data.as_str()),
            ("facade.mwy", "m:@\"./data.mwy\";->m"),
        ];
        let separate = (0..20)
            .map(|id| format!("<T{id}>:{{n:m.width;-><int32[n]>}};"))
            .collect::<String>();
        let repeated = (0..20)
            .map(|id| format!("n{id}:m.width;"))
            .collect::<String>();
        for profile in ["debug", "release"] {
            let output =
                super::file_modules::case(&format!("m:@\"./facade.mwy\";{separate}"), &files)
                    .command("check", &["--profile", profile, "--json"]);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let output = super::file_modules::case(
                &format!("m:@\"./facade.mwy\";<T>:{{{repeated}-><int32>}}"),
                &files,
            )
            .command("check", &["--profile", profile, "--json"]);
            let error = String::from_utf8_lossy(&output.stderr);
            if heavy {
                assert_eq!(output.status.code(), Some(1));
                assert!(error.contains("\"code\":\"B001\""), "{error}");
                assert!(error.contains("computed type bootstrap budget"), "{error}");
            } else {
                assert!(output.status.success(), "{error}");
            }
        }
    }
}

#[test]
pub(crate) fn record_scratch_preserves_imported_records_staging_and_namespace_gates() {
    let case = super::file_modules::case(
        "m:@\"./facade.mwy\";d:@\"debug\";<T>:{n<uint8>:m;-><int32[n+m.width]>};v<T>:[7];d.print(\"entry\");d.print(v[1]);d.print(m+0)",
        &[
            (
                "data.mwy",
                "d:@\"debug\";d.print(\"data\");->row:{->part:{->n<uint8>:4};->other:1}",
            ),
            (
                "facade.mwy",
                "m:@\"./data.mwy\";d:@\"debug\";n<uint8>:{row:m.row;part:row.part;ok:{copy:m.row;->copy.part.n==4};|ok|->part.n;|!ok|->2;unused:{->n:0}};->n;->width:n;d.print(\"facade\")",
            ),
        ],
    );
    for action in ["check", "build"] {
        for profile in ["debug", "release"] {
            let output = case.command(action, &["--profile", profile]);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(output.stdout.is_empty());
            assert!(output.stderr.is_empty());
        }
    }
    case.runs(b"data\nfacade\nentry\n7\n4\n");
    for body in [
        "m:@\"./data.mwy\";n:{copy:m;->copy.width}",
        "n:{m:@\"./data.mwy\";->m.width}",
    ] {
        let source = format!("{body};<T>:{{-><int32[n]>}};v<T>:[7];d:@\"debug\";d.print(v[1])");
        super::file_modules::case(&source, &[("data.mwy", "->width:4")]).runs(b"7\n");
    }
    for source in [
        "m:@\"./data.mwy\";n:{copy:{->row:m};->copy.row.width};<T>:{v:n;-><int32>}",
        "m:@\"./data.mwy\";n:{copy:{->m};->copy.width};<T>:{v:n;-><int32>}",
    ] {
        let output = super::file_modules::case(source, &[("data.mwy", "->width:4")])
            .command("check", &["--json"]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{source}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E211\""), "{source}: {error}");
    }
}
