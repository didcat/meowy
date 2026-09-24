use super::*;

#[test]
pub fn emitted_slot_reads_and_writes_share_returned_storage() {
    Case::new(
        r#"
d:@"debug"
initial<int32>:(){d.print("init");->1}
value:{->n:=initial();copy:n;n=n+1;d.print(copy);d.print(n);->label:"kept"}
d.print(value.n);d.print(value.label)
<Row>:<{n<int32>:=;label<string>}>
typed<Row>:{->n:=3;n=4;->label:"typed"}
d.print(typed.n)
<A>:<{n<int32>:=}>
<B>:<{text<string>}>
make<A><B>:(){->n:=1;n=2}
selected:make()
|selected<A>|d.print(selected.n)
"#,
    )
    .runs(b"init\n1\n2\n2\nkept\n4\n2\n");
}

#[test]
pub fn emitted_slot_aggregate_and_mixed_paths_publish_current_values() {
    Case::new(
        r#"
d:@"debug"
<Item>:<{n<int32>:=}>
value:{
    ->items<int32[3]>:=[1,2]
    items[2]=9
    items=items.add(3)
    ->child:={->n:=4;->rows<Item[2]>:=[{->n:=5}]}
    child.n=6
    child.rows[1].n=7
    d.print(items[2]);d.print(child.n);d.print(child.rows[1].n)
}
d.print(value.items.size());d.print(value.items[2]);d.print(value.items[3])
d.print(value.child.n);d.print(value.child.rows[1].n)
"#,
    )
    .runs(b"9\n6\n7\n3\n9\n3\n6\n7\n");
}

#[test]
pub fn emitted_slot_views_convert_widened_and_optional_storage() {
    Case::new(
        r#"
d:@"debug"
choose:(flag<boolean>)'result{
    |flag|{'result->n:=1;n=2;d.print(n)}
    |!flag|{'result->n:="before";n="after";d.print(n)}
}
one:choose(true).n
two:choose(false).n
|one<int32>|d.print(one~<int32>)
|two<string>|d.print(two~<string>)
maybe:(flag<boolean>)'result{|flag|{'result->child:={->n:=3};child.n=4;d.print(child.n)}}
present:maybe(true).child
absent:maybe(false).child
|present<{n<int32>:=}>|d.print(present.n)
|absent<null>|d.print("absent")
"#,
    )
    .runs(b"2\nafter\n2\nafter\n4\n4\nabsent\n");
}

#[test]
pub fn emitted_slot_aliases_follow_named_targets_and_restart_lifetimes() {
    Case::new(
        r#"
d:@"debug"
named:'result{{'result->n:=1;n=2;d.print(n)};'result.leave()}
d.print(named.n)
count:=0
restarted:'again{->n:=0;count=count+1;n=count;|count<2|'again.restart()}
d.print(restarted.n)
outer:{->n:=0;'loop{n=n+1;|n<3|'loop.restart()};d.print(n)}
d.print(outer.n)
left:'out{->n:=1;n=7;'out.leave();n=9}
d.print(left.n)
"#,
    )
    .runs(b"2\n2\n2\n3\n3\n7\n");
}

#[test]
pub fn discarded_emitted_slots_keep_local_effects_without_invalid_projection() {
    Case::new(
        r#"
d:@"debug"
run<null>:(flag<boolean>){
    'escape{
        value:'target{
            |flag|{'target->n:=1;n=7;d.print(n);'escape.leave()}
            ->"done"
        }
        d.print(value)
    }
}
different<null>:(flag<boolean>){
    'escape{
        value:'target{
            |flag|{'target->n:=1;n=8;d.print(n);'escape.leave()}
            ->n:="text"
        }
        d.print(value.n)
    }
}
run(true);run(false);different(true);different(false)
"#,
    )
    .runs(b"7\ndone\n8\ntext\n");
}

#[test]
pub fn emitted_slot_mutation_cannot_publish_stale_variant_or_origin_facts() {
    for source in [
        "owner:=1;view:&owner;value:{->tag<int32><null>:=null;tag=1};|value.tag<int32>|{owner=2;copy:*view}",
        "owner:=1;view:&owner;value:{->child:={->tag<int32><null>:=null};child.tag=1};|value.child.tag<int32>|{owner=2;copy:*view}",
        "owner:=1;value:{->view:&owner;->tag<int32><null>:=null;tag=1};owner=2;copy:*(value.view)",
        "value:{->items<int32[2]>:=[1];items[{items=[];->1}]=2}",
        "value:{->items<int32[2]>:=[1];items[1]={items=[];->2}}",
    ] {
        let output = Case::new(source).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E302\""), "{source}: {error}");
    }
}

#[test]
pub fn emitted_slot_aliases_keep_type_and_ownership_boundaries_explicit() {
    for (source, code) in [
        ("value:{->n:1;n=2}", "E305"),
        ("value:{->n:=1;n=\"wrong\"}", "E207"),
        ("value:{->n:=1;->n:=2}", "E205"),
        (
            "owner:1;n:=2;value:'out{'loop{'out->view:=&owner;view=&owner;n=n-1;|n>0|'loop.restart()}}",
            "B001",
        ),
        ("value:{->n:=1;f<int32>:(){->n}}", "B001"),
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
pub fn emitted_slot_failures_preserve_runtime_widths_and_rhs_effects() {
    for (source, expression, stdout, code, message) in [
        (
            "d:@\"debug\";run<null>:(flag<boolean>){value:'result{|flag|{'result->n<uint8>:=255;d.print(\"value\");n=n+1}}};run(true)",
            "n+1",
            "value\n",
            "P002",
            "uint8 + overflow (left 255, right 1; range 0..255)",
        ),
        (
            "d:@\"debug\";value:{->n:=1;n={d.print(\"rhs\");d.panic(\"stop\")}}",
            "d.panic(\"stop\")",
            "rhs\n",
            "P006",
            "stop",
        ),
    ] {
        let start = source.rfind(expression).unwrap();
        let end = start + expression.len();
        let case = Case::new(source);
        for profile in ["debug", "release"] {
            let output = case.command("run", &["--profile", profile]);
            assert_eq!(output.status.code(), Some(1), "{source}");
            assert_eq!(output.stdout, stdout.as_bytes());
            assert_eq!(
                output.stderr,
                format!("panic[{code}]: {message} at bytes {start}..{end}\n").as_bytes()
            );
        }
    }
}
