use super::{Case, file_modules::case};

#[test]
pub(crate) fn logical_type_expressions_keep_facade_types_and_startup() {
    case(
        "m:@\"./facade.mwy\";d:@\"debug\";<T>:{kind:((m).Kind);copy:kind!<null>;|false|unused:<int32[1/0]>;->copy};v<T>:7;d.print(v)",
        &[
            ("data.mwy", "d:@\"debug\";d.print(1);->Kind<Type>:<uint8><null>"),
            ("facade.mwy", "m:@\"./data.mwy\";d:@\"debug\";d.print(2);->Kind<Type>:m.Kind"),
        ],
    ).runs(b"1\n2\n7\n");
    Case::new("d:@\"debug\";f<uint8>:(n<uint8>){<T>:((n/0)<>!<null>);v<T>:7;->v};d.print(f(2))")
        .runs(b"7\n");
}

#[test]
pub(crate) fn logical_type_expressions_keep_source_errors_through_facades() {
    let source = "->bad<uint8>:255+1";
    let case = case(
        "m:@\"./facade.mwy\";<T>:(({n:m.bad;-><uint8>})!<null>)",
        &[
            ("data.mwy", source),
            ("facade.mwy", "m:@\"./data.mwy\";->bad:m.bad"),
        ],
    );
    for action in ["check", "build", "run"] {
        let output = case.command(action, &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E107\""), "{error}");
        assert!(
            error.contains(&format!(
                "\"path\":\"{}\"",
                case.path.join("data.mwy").display()
            )),
            "{error}"
        );
        assert!(
            error.contains(&format!("\"start\":{}", source.find("255+1").unwrap())),
            "{error}"
        );
    }
}

#[test]
pub(crate) fn logical_record_slots_keep_facade_composition_and_native_layout() {
    case(
        "m:@\"./facade.mwy\";d:@\"debug\";<T>:{base:m.row.part;r<{n<uint8>;flag<boolean>;extra<uint8>}>:{->base;->extra:2};copy:r;-><uint8[copy.n+copy.extra]>};v<T>:[7];d.print(v[1]);d.print(m.row.part.flag)",
        &[
            ("data.mwy", "d:@\"debug\";d.print(1);->row:{->part:{->n<uint8>:2;->flag:true}}"),
            ("facade.mwy", "m:@\"./data.mwy\";d:@\"debug\";d.print(2);->row:m.row"),
        ],
    ).runs(b"1\n2\n7\ntrue\n");
    Case::new(
        "d:@\"debug\";<T>:{r<{part<{n<uint8>}>;flag<boolean>}>:{->{->part:{->n:4}};->flag:true};|false|unused:r;copy:r;-><uint8[copy.part.n]>};v<T>:[9];d.print(v[1])",
    ).runs(b"9\n");
}

#[test]
pub(crate) fn logical_record_imports_keep_copies_widths_and_startup() {
    case(
        "m:@\"./facade.mwy\";alias:m;d:@\"debug\";<T>:{copy:((alias).row).part;again:copy;|false|unused:m.row.part;-><uint8[again.n]>};v<T>:[7];d.print(v[1]);d.print(m.row.part.flag)",
        &[
            ("data.mwy", "d:@\"debug\";d.print(1);->row:{->part:{->n<uint8>:2;->flag:true}}"),
            ("facade.mwy", "m:@\"./data.mwy\";d:@\"debug\";d.print(2);->row:m.row"),
        ],
    ).runs(b"1\n2\n7\ntrue\n");
}

#[test]
pub(crate) fn logical_record_import_errors_keep_original_file_and_span() {
    let source = "row:{->part:{->n<uint8>:4};->bad<uint8>:255+1};->row:row";
    let case = case(
        "m:@\"./facade.mwy\";<T>:{copy:m.row.part;-><int32>}",
        &[
            ("data.mwy", source),
            ("facade.mwy", "m:@\"./data.mwy\";->row:m.row"),
        ],
    );
    for action in ["check", "build", "run"] {
        let output = case.command(action, &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E107\""), "{error}");
        assert!(
            error.contains(&format!(
                "\"path\":\"{}\"",
                case.path.join("data.mwy").display()
            )),
            "{error}"
        );
        assert!(
            error.contains(&format!("\"start\":{}", source.find("255+1").unwrap())),
            "{error}"
        );
    }
}

#[test]
pub(crate) fn logical_projection_imports_preserve_grouped_widths_and_startup() {
    case(
        "m:@\"./facade.mwy\";alias:m;d:@\"debug\";<T>:{n:((alias).row).part.n+alias.row.part.n;flag:alias.row.part.flag==((m.row).part).flag;-><uint8[n]>};v<T>:[7];d.print(v[1]);d.print(m.row.part.flag)",
        &[
            ("data.mwy", "d:@\"debug\";d.print(1);->row:{->part:{->n<uint8>:2;->flag:true}}"),
            ("facade.mwy", "m:@\"./data.mwy\";d:@\"debug\";d.print(2);->row:m.row"),
        ],
    ).runs(b"1\n2\n7\ntrue\n");
}

#[test]
pub(crate) fn computed_fields_keep_local_module_values_types_and_initialization_order() {
    case(
        "m:@\"./types.mwy\";d:@\"debug\";items<m.Items>:[3,7];d.print(items[2]);d.print(m.get())",
        &[("types.mwy", "d:@\"debug\";d.print(\"init\");settings:{->other<uint8>:2;->width<uint8>:other*2};alias:settings;-><Items>:{-><int32[alias.width]>};->get<int32>:(){<Local>:{n:settings.width;-><int32[n]>};items<Local>:[4,8];->items[2]}")],
    ).runs(b"init\n7\n8\n");
}

#[test]
pub(crate) fn computed_fields_preserve_errors_in_unselected_siblings() {
    let source = "#é#\n|false|{row<{good<uint8>;bad<uint8>}>:{->good<uint8>:4;->bad<uint8>:255+1};<T>:{n:row.good;-><int32>}}";
    let case = case("m:@\"./types.mwy\"", &[("types.mwy", source)]);
    for action in ["check", "build", "run"] {
        let output = case.command(action, &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E107\""), "{error}");
        assert!(
            error.contains(&format!(
                "\"path\":\"{}\"",
                case.path.join("types.mwy").display()
            )),
            "{error}"
        );
        assert!(
            error.contains(&format!("\"start\":{}", source.find("255+1").unwrap())),
            "{error}"
        );
    }
}

#[test]
pub(crate) fn computed_fields_do_not_execute_effectful_record_initializers() {
    let source = "d:@\"debug\";settings:{->width:4;d.print(\"must not run\")};<T>:{n:settings.width;-><int32[n]>}";
    let case = Case::new(source);
    for action in ["check", "build", "run"] {
        let output = case.command(action, &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E211\""));
    }
}

#[test]
pub(crate) fn computed_fields_charge_the_whole_record_work_on_every_read() {
    let tail = (1..110)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let source = format!("settings:{{->width:4;v0:1;{tail}}};<T>:{{n:settings.width;-><int32>}}");
    let output = Case::new(&source).command("check", &["--json"]);
    assert_eq!(output.status.code(), Some(1));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("\"code\":\"B001\""), "{error}");
    assert!(error.contains("computed type bootstrap budget"), "{error}");
}

#[test]
pub(crate) fn nested_records_preserve_module_initialization_and_subrecord_aliases() {
    case(
        "m:@\"./types.mwy\";d:@\"debug\";items<m.Items>:[3,7];d.print(items[2]);d.print(m.get())",
        &[("types.mwy", "d:@\"debug\";d.print(\"init\");settings:{->limits:{->width<uint8>:4}};alias:settings.limits;-><Items>:{-><int32[alias.width]>};->get<int32>:(){<Local>:{n:settings.limits.width;-><int32[n]>};v<Local>:[4,8];->v[2]}")],
    ).runs(b"init\n7\n8\n");
}

#[test]
pub(crate) fn nested_records_keep_errors_outside_a_projected_subrecord() {
    let source = "#é#\npart:{->n<uint8>:4};|false|{row<{nested<{n<uint8>}>;bad<uint8>}>:{->nested:part;->bad<uint8>:255+1};alias:row.nested;<T>:{n:alias.n;-><int32>}}";
    let case = case("m:@\"./types.mwy\"", &[("types.mwy", source)]);
    for action in ["check", "build", "run"] {
        let output = case.command(action, &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E107\""), "{error}");
        assert!(
            error.contains(&format!(
                "\"path\":\"{}\"",
                case.path.join("types.mwy").display()
            )),
            "{error}"
        );
        assert!(
            error.contains(&format!("\"start\":{}", source.find("255+1").unwrap())),
            "{error}"
        );
    }
}

#[test]
pub(crate) fn nested_records_cannot_drop_ancestor_effects_or_work() {
    let source = "d:@\"debug\";row:{->nested:{->n:4};d.print(\"must not run\")};alias:row.nested;<T>:{n:alias.n;-><int32>}";
    let output = Case::new(source).command("run", &["--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E211\""));
    let tail = (1..110)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let source = format!(
        "row:{{->nested:{{->n:4}};v0:1;{tail}}};alias:row.nested;<T>:{{n:alias.n;-><int32>}}"
    );
    let output = Case::new(&source).command("check", &["--json"]);
    assert_eq!(output.status.code(), Some(1));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("\"code\":\"B001\""), "{error}");
    assert!(error.contains("computed type bootstrap budget"), "{error}");
}

#[test]
pub(crate) fn imported_inputs_support_fields_copies_reexports_and_function_types() {
    case(
        "m:@\"./facade.mwy\";alias:m;copy:alias.row.nested;leaf:copy.n;d:@\"debug\";<T>:{n:m.width+leaf;-><int32[n]>};v<T>:[3,7];d.print(v[2]);d.print(m.get());d.print(m.row.nested.n)",
        &[
            ("data.mwy", "base<uint8>:2;->width:base*2;->row:{->unused:3;->nested:{->n:width}}"),
            ("facade.mwy", "m:@\"./data.mwy\";->width:m.width;->row:m.row;->get<int32>:(){<T>:{n:m.row.nested.n;-><int32[n]>};v<T>:[9];->v[1]}"),
        ],
    ).runs(b"7\n9\n4\n");
}

#[test]
pub(crate) fn imported_inputs_reject_effects_private_names_and_runtime_captures() {
    for (source, module, code) in [
        (
            "m:@\"./data.mwy\";<T>:{n:m.width;-><int32>}",
            "d:@\"debug\";->width:{->4;d.print(1)}",
            "E211",
        ),
        (
            "m:@\"./data.mwy\";copy:m.width;<T>:{n:copy;-><int32>}",
            "d:@\"debug\";->width:{->4;d.print(1)}",
            "E211",
        ),
        (
            "m:@\"./data.mwy\";<T>:{n:m.width;-><int32>}",
            "|true|->width:4",
            "E211",
        ),
        (
            "m:@\"./data.mwy\";<T>:{n:m.hidden;-><int32>}",
            "hidden:4;->width:hidden",
            "E201",
        ),
        (
            "m:@\"./data.mwy\";alias:m.row;<T>:{n:alias.n;-><int32>}",
            "d:@\"debug\";->row:{->n:4;d.print(1)}",
            "E211",
        ),
        (
            "m:@\"./data.mwy\";f<int32>:(){->m.row.n}",
            "->row:{->n:4}",
            "B001",
        ),
        (
            "m:@\"./data.mwy\";alias:m.row;f<int32>:(){->alias.n}",
            "->row:{->n:4}",
            "B001",
        ),
        (
            "row:{->nested:{->n:4}};f<int32>:(){->row.nested.n}",
            "->width:4",
            "B001",
        ),
    ] {
        let case = case(source, &[("data.mwy", module)]);
        let output = case.command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(&format!("\"code\":\"{code}\"")),
            "{source}: {error}"
        );
    }
}
