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
        "row:{->n:4;->flag:true};->row.n",
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
