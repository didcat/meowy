use super::Case;

#[test]
pub fn list_union_literals_select_unique_types_capacities_and_widths() {
    Case::new(
        r#"
d:@"debug"
values<int32[1]><string[1]>:[1]
words<int32[1]><string[1]>:["one"]
|values<int32[1]>|d.print(values[1])
|words<string[1]>|d.print(words[1])
two<int32[1]><int32[2]>:[11,22]
|two<int32[2]>|{d.print(two.size());d.print(two[2])}
wide<uint8[1]><uint16[1]>:[300]
|wide<uint16[1]>|d.print(wide[1])
byte<uint8>:7
typed<uint8[2]><uint16[2]>:[8,byte]
|typed<uint8[2]>|{d.print(typed[1]);d.print(typed[2])}
large<uint64>:9
late<int32[2]><uint64[2]>:[4294967295,large]
|late<uint64[2]>|{d.print(late[1]);d.print(late[2])}
optional<int32[1]><string[1]><null>:[4]
|optional<int32[1]>|d.print(optional[1])
<Value>:<int32><string>
member<Value>:5
existing<Value[1]><int32[1]>:[member]
|existing<Value[1]>|{element:existing[1];|element<int32>|d.print(element~<int32>)}
record:{->23;->tag:"record"}
projected<int32[1]><string[1]>:[record]
|projected<int32[1]>|d.print(projected[1])
empty<uint8[0]><uint8[0]>:[]
d.print(empty.size())
"#,
    )
    .runs(b"1\none\n2\n22\n300\n8\n7\n4294967295\n9\n4\n5\n23\n0\n");
}

#[test]
pub fn list_union_literals_preserve_nested_and_record_contexts() {
    Case::new(
        r#"
d:@"debug"
nested<int32[2][2]><string[2][2]>:[[1],[2,3]]
|nested<int32[2][2]>|{d.print(nested[2][2]);d.print(nested[1].size())}
inner<int32[2]>:[7]
typed<int32[1][1]><int32[2][1]>:[inner]
|typed<int32[2][1]>|d.print(typed[1][1])
numeric<uint8[2][1]><uint16[2][1]>:[[300]]
|numeric<uint16[2][1]>|d.print(numeric[1][1])
<Row>:<{value<int32>;name<string>}>
<Other>:<{message<string>}>
row<Row>:{->value:17;->name:"row"}
rows<Row[2]><Other[2]>:[row]
|rows<Row[2]>|{d.print(rows[1].name);d.print(rows[1].value)}
records<Row[1]><Row[2]>:[{->value:1;->name:"first"},{->value:2;->name:"second"}]
|records<Row[2]>|d.print(records[2].name)
<A>:<{value<int64>;tag<string><null>}>
<B>:<{value<string>}>
fresh<A[1]><B[1]>:[{->value:7}]
accept64<int64>:(value<int64>){->value}
|fresh<A[1]>|{d.print(accept64(fresh[1].value));|fresh[1].tag<null>|d.print("missing tag")}
"#,
    )
    .runs(b"3\n1\n7\n300\nrow\n17\nsecond\n7\nmissing tag\n");
}

#[test]
pub fn list_union_literals_reject_ambiguous_or_impossible_contexts() {
    for (source, code) in [
        ("values<uint8[1]><uint16[1]>:[1]", "E207"),
        ("values<int32[2]><int32[3]>:[1]", "E207"),
        ("values<int32[0]><string[2]>:[]", "E207"),
        ("values<int32[2]><string[2]>:[true]", "E207"),
        ("byte<uint8>:7;values<uint16[1]><string[1]>:[byte]", "E207"),
        ("values<int32[1]><string[2]>:[1,2,3]", "E103"),
        ("values<int32[1]><string[2]>:[1,2]", "E207"),
        ("values<uint8[1]><int8[1]>:[256]", "E207"),
        ("values<uint8[1]>:[256]", "E216"),
        ("values<int32[1][1]><int32[2][1]>:[[1]]", "E207"),
        (
            "<Value>:<int32><string>;values<Value[1]><int32[1]>:[1]",
            "E207",
        ),
        ("values<int32[2]><string[2]>:[1,missing]", "E201"),
        ("values<int32[2]><string[2]>:[1,1/0]", "E107"),
    ] {
        let case = Case::new(source);
        for profile in ["debug", "release"] {
            let result = case.command("build", &["--json", "--profile", profile]);
            assert_eq!(result.status.code(), Some(1), "{source}");
            let error = String::from_utf8_lossy(&result.stderr);
            assert!(
                error.contains(&format!("\"code\":\"{code}\"")),
                "{source}: {error}"
            );
            assert!(!case.path.join("build").exists());
        }
    }
}

#[test]
pub fn list_union_candidate_checks_preserve_effect_order_and_single_evaluation() {
    Case::new(
        r#"
d:@"debug"
mark<int32>:(value<int32>){d.print(value);->value}
selected<int32[3]><string[3]>:[mark(1),mark(2)]
|selected<int32[3]>|d.print(selected[1]+selected[2])
wide<uint64>:(){d.print("wide");->7}
large<uint64[2]><string[2]>:[4294967295,wide()]
|large<uint64[2]>|{d.print(large[1]);d.print(large[2])}
count:=0
ordered<int32[1]><int32[2]>:[{d.print("first");count=count+1;->count},{d.print("second");count=count+1;->count}]
|ordered<int32[2]>|{d.print(ordered[1]);d.print(ordered[2])}
d.print(count)
"#,
    )
    .runs(b"1\n2\n3\nwide\n4294967295\n7\nfirst\nsecond\n1\n2\n2\n");
}

#[test]
pub fn list_union_candidate_checks_preserve_early_exits_and_panics() {
    Case::new(
        r#"
d:@"debug"
'out {
    unused<int32[1]><int32[2]>:[{d.print("before");'out.leave();->1},2]
    d.print("unreachable")
}
d.print("done")
"#,
    )
    .runs(b"before\ndone\n");
    let source =
        "d:@\"debug\";values<int32[1]><int32[2]>:[1,{d.print(\"element\");d.panic(\"stop\")}]";
    let case = Case::new(source);
    for profile in ["debug", "release"] {
        let result = case.command("run", &["--profile", profile]);
        assert_eq!(result.status.code(), Some(1));
        assert_eq!(result.stdout, b"element\n");
        let start = source.find("d.panic(\"stop\")").unwrap();
        let end = start + "d.panic(\"stop\")".len();
        assert_eq!(
            result.stderr,
            format!("panic[P006]: stop at bytes {start}..{end}\n").as_bytes()
        );
    }
}

#[test]
pub fn compound_list_candidates_preserve_intermediate_widths_and_typed_leaves() {
    Case::new(
        r#"
d:@"debug"
grouped<int8[1]><int16[1]>:[-(128)]
|grouped<int16[1]>|d.print(grouped[1])
intermediate<int8[1]><int16[1]>:[(127+1)-1]
|intermediate<int16[1]>|d.print(intermediate[1])
negative<int8[1]><int16[1]>:[-(-128)]
|negative<int16[1]>|d.print(negative[1])
bits<int8[1]><uint8[1]>:[(@"bits").not(128)]
|bits<uint8[1]>|d.print(bits[1])
byte<uint8>:1
typed<uint8[1]><int16[1]>:[byte+1]
|typed<uint8[1]>|d.print(typed[1])
wide<uint64>:18446744073709551615
exact<uint64[1]><int64[1]>:[wide-1]
|exact<uint64[1]>|d.print(exact[1])
"#,
    )
    .runs(b"-128\n127\n128\n127\n2\n18446744073709551614\n");
}

#[test]
pub fn compound_list_candidates_keep_short_circuit_and_float_rules() {
    Case::new(
        r#"
d:@"debug"
flags<boolean[2]><int32[2]>:[false&&(1/0==0),true||(1/0==0)]
|flags<boolean[2]>|{d.print(flags[1]);d.print(flags[2])}
core:@"core"
yes:core.true
logic<boolean[1]><string[1]>:[yes&&("a"<"b")]
|logic<boolean[1]>|d.print(logic[1])
wide<float32[1]><float64[1]>:[1e39-1e39]
|wide<float64[1]>|d.print(wide[1]==0.0)
seed<float32>:1.0
rounded<float32[2]><float64[2]>:[1.0000000596046447753906250000000001+0.0,seed]
expected<float32>:1.00000011920928955078125
|rounded<float32[2]>|d.print(rounded[1]==expected)
"#,
    )
    .runs(b"false\ntrue\ntrue\ntrue\ntrue\n");
}

#[test]
pub fn compound_list_candidates_resolve_nested_literals_and_record_fields() {
    Case::new(
        r#"
d:@"debug"
values<uint8[1][1]><uint16[1][1]>:[[250+10]]
|values<uint16[1][1]>|d.print(values[1][1])
<Small>:<{value<int8>}>
<Wide>:<{value<int16>}>
rows<Small[1]><Wide[1]>:[{->value:(127+1)-1}]
|rows<Wide[1]>|d.print(rows[1].value)
"#,
    )
    .runs(b"260\n127\n");
}

#[test]
pub fn compound_list_resolution_does_not_replay_effectful_elements() {
    Case::new(
        r#"
d:@"debug"
count:=0
values<int8[2]><uint8[2]>:[127+1,{d.print("after");count=count+1;->1}]
|values<uint8[2]>|{d.print(values[1]);d.print(values[2])}
d.print(count)
mark<uint8>:(){d.print("typed");->7}
later<uint8[2]><uint16[2]>:[1+1,mark()]
|later<uint8[2]>|{d.print(later[1]);d.print(later[2])}
"#,
    )
    .runs(b"after\n128\n1\n1\ntyped\n2\n7\n");
}

#[test]
pub fn compound_list_candidates_preserve_ambiguity_and_source_errors() {
    for (source, code) in [
        ("values<int8[1]><int16[1]>:[1+1]", "E207"),
        ("values<int8[1]><uint8[1]>:[(@\"bits\").not(1)]", "E207"),
        ("values<int8[1]><uint8[1]>:[-(128)]", "E207"),
        ("values<int8[1]><int16[1]>:[1/0]", "E207"),
        ("values<float32[1]><float64[1]>:[3e38+3e38]", "E207"),
        ("byte<int8>:127;values<int8[1]><int16[1]>:[byte+1]", "E107"),
        ("values<int8[1]>:[(127+1)-1]", "E107"),
        ("values<int8[1]>:[-(128)]", "E216"),
        ("values<int8[2]><string[2]>:[1,missing+1]", "E201"),
        (
            "value<uint8>:1;values<uint8[1]><uint16[1]>:[{d:@\"debug\";d.print(1);->1}]",
            "E207",
        ),
        (
            "outer<int8>:1;f<null>:(){values<int8[1]><int16[1]>:[outer+1]}",
            "B001",
        ),
    ] {
        let case = Case::new(source);
        for profile in ["debug", "release"] {
            let output = case.command("build", &["--json", "--profile", profile]);
            assert_eq!(output.status.code(), Some(1), "{source}");
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(
                error.contains(&format!("\"code\":\"{code}\"")),
                "{source}: {error}"
            );
            assert!(!case.path.join("build").exists());
        }
    }
}

#[test]
pub fn compound_list_probes_respect_saved_reach_and_runtime_inputs() {
    let prefix = r#"d:@"debug";stop<never>:(){d.print("stop");d.panic("end")};"#;
    let source = format!("{prefix}values<int8[2]><int16[2]>:[stop(),(127+1)-1]");
    let output = Case::new(&source).command("check", &["--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E207\""));
    let source = format!("{prefix}values<int8[2]><int16[2]>:[(127+1)-1,stop()]");
    let start = source.find("d.panic(\"end\")").unwrap();
    let end = start + "d.panic(\"end\")".len();
    let case = Case::new(&source);
    for profile in ["debug", "release"] {
        let output = case.command("run", &["--profile", profile]);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, b"stop\n");
        assert_eq!(
            output.stderr,
            format!("panic[P006]: end at bytes {start}..{end}\n").as_bytes()
        );
    }
    let source = "number<int8>:=127;values<int8[1]><int16[1]>:[number+1]";
    let start = source.find("number+1").unwrap();
    let case = Case::new(source);
    for profile in ["debug", "release"] {
        let output = case.command("run", &["--profile", profile]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, format!("panic[P002]: int8 + overflow (left 127, right 1; range -128..127) at bytes {start}..{}\n", start + "number+1".len()).as_bytes());
    }
}
