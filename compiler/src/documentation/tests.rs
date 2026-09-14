use super::*;

#[test]
pub(crate) fn documentation_attaches_and_derives_checked_signatures() {
    let source = "#!| Module links to [[add]]. |!#\n#| Add [[left]] and [[right]] using [[<int32>]]. |#\nadd<int32>:(#| Left. |#left<int32>,#| Right. |#right<int32>){->left+right}";
    let (_, model) = checked(source, true).unwrap();
    let model = model.unwrap();
    assert_eq!(model.entries.len(), 4);
    assert_eq!(model.entries[1].signature, "(int32,int32)->int32");
    assert_eq!(model.entries[2].signature, "int32");
    assert!(model.entries.iter().all(|entry| entry.checked));
    model.require_public().unwrap();
}

#[test]
pub(crate) fn documentation_diagnoses_orphan_duplicate_trailing_and_module_placement() {
    for source in [
        "#| orphan |#->1",
        "#| first |# #| second |#x:1",
        "x:1 #| trailing |#\ny:2",
        "x:1\n#!| late |!#",
        "#!| first |!#\n#!| second |!#",
        "|false|{#| orphan |#}",
    ] {
        assert_eq!(
            checked(source, true).unwrap_err()[0].code,
            "E801",
            "{source}"
        );
    }
}

#[test]
pub(crate) fn documentation_resolves_scoped_links_and_record_members_without_reads() {
    let source = "<R>:<{#| Field of [[<R>]]. |#n<int32>}>;r<R>:{->n:7};#| Read [[r.n]] through its actual type. |#v:r.n";
    let (_, model) = checked(source, true).unwrap();
    assert!(
        model
            .unwrap()
            .entries
            .iter()
            .any(|entry| entry.kind == Kind::Field && entry.signature == "int32")
    );
    for source in [
        "#| [[missing]] |#x:1",
        "x:1;#| [[x.nope]] |#y:2",
        "#| [[<Missing>]] |#x:1",
        "#| [[later]] |#x:1;later:2",
        "#| [[inside]] |#f<int32>:(){inside:1;->inside}",
    ] {
        assert_eq!(
            checked(source, true).unwrap_err()[0].code,
            "E802",
            "{source}"
        );
    }
    checked("d:@\"debug\";#| [[d.print|printing]] |#alias:d.print", true).unwrap();
}

#[test]
pub(crate) fn documentation_ignores_links_in_code_and_rejects_malformed_links() {
    checked(
        "#||\nLiteral `[[unknown]]`.\n```text\n[[also_unknown]]\n```\n||#x:1",
        true,
    )
    .unwrap();
    for source in [
        "#| [[x()]] |#x:1",
        "#| [[missing |#x:1",
        "#| [[x|bad|label]] |#x:1",
    ] {
        assert_eq!(checked(source, true).unwrap_err()[0].code, "E802");
    }
}

#[test]
pub(crate) fn documentation_checks_example_metadata_without_executing_examples() {
    let source = "#||\n```meowy run\nd:@\"debug\";d.panic(\"not run\")\n```\n```output\nanything\n```\n||#x:1";
    let (_, model) = checked(source, true).unwrap();
    let model = model.unwrap();
    assert_eq!(model.entries[1].examples.len(), 1);
    assert!(!model.entries[1].examples[0].ran);
    for body in [
        "```meowy mystery\nx:1\n```",
        "```meowy reject=B001\nx:1\n```",
        "```output\nx\n```",
    ] {
        assert_eq!(
            checked(&format!("#||\n{body}\n||#x:1"), true).unwrap_err()[0].code,
            "E803"
        );
    }
}

#[test]
pub(crate) fn documentation_preserves_normalized_source_mapping_and_interpolation() {
    let source = "#||\r\n    Unicode \u{e9} [[missing]].\r\n||#x:1";
    let error = checked(source, true).unwrap_err().remove(0);
    assert_eq!(error.code, "E802");
    assert_eq!(error.span.start, source.find("[[missing]]").unwrap());
    checked(
        "d:@\"debug\";d.print(\"value {{#| Inner. |#x:7;->x}}\")",
        true,
    )
    .unwrap();
}

#[test]
pub(crate) fn documentation_renderer_escapes_active_content_and_keeps_source_private() {
    let source = "#||\n<script>alert(1)</script>\n\n[bad](javascript:alert(1))\n![image](https://example.test/secret)\n||#x:1";
    let (_, model) = checked(source, true).unwrap();
    let page = render::page(&model.unwrap(), "<entry>");
    assert!(!page.contains("<script>"));
    assert!(!page.contains("href=\"javascript:"));
    assert!(!page.contains("<img"));
    assert!(page.contains("&lt;entry&gt;"));
    assert!(page.contains("@media"));
    assert!(page.contains("&lt;entry&gt; | meowy API</title>"));
    assert!(page.contains("<p>meowy / API REFERENCE</p>"));
    assert!(page.contains("Generated locally by the meowy bootstrap."));
}

#[test]
pub(crate) fn documentation_public_policy_and_budgets_fail_explicitly() {
    let (_, model) = checked("x:1", true).unwrap();
    assert_eq!(model.unwrap().require_public().unwrap_err().code, "E803");
    let source = format!("#|{}|#x:1", "x".repeat(MAX_SOURCE));
    assert_eq!(checked(&source, true).unwrap_err()[0].code, "B001");
}

#[test]
pub(crate) fn exported_types_documentation_preserves_type_roles_and_declaration_spans() {
    let source = "#| Public counter. |#\n-><Count>:<int32>";
    let (_, model) = checked(source, true).unwrap();
    let model = model.unwrap();
    let entry = model
        .entries
        .iter()
        .find(|entry| entry.name == "Count")
        .unwrap();
    assert_eq!(entry.kind, Kind::Type);
    assert!(entry.public);
    assert!(entry.checked);
    assert_eq!(entry.signature, "int32");
    assert_eq!(entry.span.start, source.find("->").unwrap());
    model.require_public().unwrap();
}

#[test]
pub(crate) fn documentation_checks_shifted_sources_and_maps_normalized_links() {
    let source = "#!| [[f]] |!#\r\n#|| É [[x]]. ||#f<int32>:(#| Input. |#x<int32>){->x};<R>:<{#| Field. |#n<int32>}>";
    let base = 4097;
    let parsed = crate::parser::parse_documented_at(source, base).unwrap();
    let model = Model::at(source, &parsed, base, true).unwrap();
    let (_, model) = crate::check::check_documented(&parsed.block, Some(model)).unwrap();
    let model = model.unwrap();
    assert!(model.entries.iter().all(|entry| entry.checked));
    let link = &model.entries[1].links[0];
    assert_eq!(link.span.start, base + source.find("[[x]]").unwrap());
    assert_eq!(model.entries[1].signature, "(int32)->int32");
    assert_eq!(model.entries.last().unwrap().signature, "int32");
}

#[test]
pub(crate) fn documentation_shifted_attachment_and_markup_errors_keep_source_spans() {
    for source in [
        "#| One. |# #| Two. |#x:1",
        "x:1 #| Trailing. |#\ny:2",
        "#| [[x()]] |#x:1",
    ] {
        let parsed = crate::parser::parse_documented(source).unwrap();
        let local = Model::new(source, &parsed).unwrap_err();
        let parsed = crate::parser::parse_documented_at(source, 512).unwrap();
        let shifted = Model::at(source, &parsed, 512, true).unwrap_err();
        assert_eq!(shifted.code, local.code);
        assert_eq!(shifted.span.start, local.span.start + 512);
        assert_eq!(shifted.span.end, local.span.end + 512);
    }
}

#[test]
pub(crate) fn documentation_file_exports_keep_signatures_parameters_and_visibility() {
    let source = "private:7;<Hidden>:<int32>;#| Public type. |#-><Count>:<int32>;#| Increment [[x]] using [[<Count>]]. |#->inc<Count>:(#| Input. |#x<Count>){->x+1};#| Alias [[inc]]. |#->alias<(int32)->int32>:inc";
    let parsed = crate::parser::parse_documented(source).unwrap();
    let model = Model::at(source, &parsed, 0, false).unwrap();
    let (_, model) = crate::check::check_documented(&parsed.block, Some(model)).unwrap();
    let model = model.unwrap();
    model.require_public().unwrap();
    for name in ["private", "Hidden"] {
        assert!(
            !model
                .entries
                .iter()
                .find(|entry| entry.name == name)
                .unwrap()
                .public
        );
    }
    let function = model
        .entries
        .iter()
        .find(|entry| entry.name == "inc")
        .unwrap();
    assert_eq!(function.kind, Kind::Function);
    assert_eq!(function.signature, "(int32)->int32");
    assert!(function.checked);
    let param = model
        .entries
        .iter()
        .find(|entry| entry.name == "x")
        .unwrap();
    assert_eq!(param.signature, "int32");
    assert!(param.checked);
}

#[test]
pub(crate) fn documentation_public_file_exports_reject_private_links() {
    for source in [
        "hidden:7;#| [[hidden]] |#->value:1",
        "<Hidden>:<int32>;#| [[<Hidden>]] |#-><Public>:<int32>",
        "hidden<int32>:(){->1};#| [[hidden]] |#->f<int32>:(){->1}",
    ] {
        let parsed = crate::parser::parse_documented(source).unwrap();
        let model = Model::at(source, &parsed, 0, false).unwrap();
        let error = crate::check::check_documented(&parsed.block, Some(model)).unwrap_err();
        assert_eq!(error[0].code, "E802", "{source}");
        assert!(error[0].message.contains("private"), "{error:?}");
    }
    let source = "hidden:1;#| [[hidden]] |#private:2;#| Public. |#->value:3";
    let parsed = crate::parser::parse_documented(source).unwrap();
    let model = Model::at(source, &parsed, 0, false).unwrap();
    crate::check::check_documented(&parsed.block, Some(model)).unwrap();
}

#[test]
pub(crate) fn documentation_retains_explicit_type_call_fields_and_computed_bindings() {
    let source =
        r#"result:f<{#| Member. |#n<int32>}>();other:f<({#| Type. |#kind:<uint32>;->kind})>()"#;
    let base = 513;
    let parsed = crate::parser::parse_documented_at(source, base).unwrap();
    let model = Model::at(source, &parsed, base, true).unwrap();
    for (name, token) in [("n", "n<int32>"), ("kind", "kind:<uint32>")] {
        let entry = model
            .entries
            .iter()
            .find(|entry| entry.name == name)
            .unwrap();
        assert!(entry.doc.is_some());
        assert_eq!(entry.span.start, base + source.find(token).unwrap());
    }
    let field = model
        .entries
        .iter()
        .find(|entry| entry.name == "n")
        .unwrap();
    assert_eq!(model.entries[field.parent.unwrap()].name, "result");
}
