use super::Case;

#[test]
pub fn nullable_references_preserve_absence_and_last_use() {
    Case::new(
        r#"
d:@"debug"
inspect<null>:(flag<boolean>){
    owner:=7
    view<&int32><null>:{|flag|->&owner}
    empty<&int32><null>:null
    d.print(view==empty)
    |view<null>|owner=9
    |view<&int32>|d.print(*(view~<&int32>))
    owner=10
    d.print(owner)
    record<{nested<{view<&int32><null>}>;count<int32>}>:{->nested:{|flag|->view:&owner};->count:3}
    |record.nested.view<&int32>|d.print(*(record.nested.view~<&int32>))
    owner=11
    d.print(record.count)
}
inspect(true)
inspect(false)
"#,
    )
    .runs(b"false\n7\n10\n10\n3\ntrue\n10\n3\n");
}

#[test]
pub fn reference_union_retagging_keeps_member_and_field_origins() {
    Case::new(
        r#"
d:@"debug"
<Row>:<{view<&int32>;extra<int32>}>
<Text>:<{view<&string>}>
inspect<null>:(flag<boolean>){
    left:=41
    right:="original"
    value<Row><Text>:{|flag|->{->view:&left;->extra:2};|!flag|->{->view:&right}}
    wide<Row><Text><null><boolean>:value
    |wide<Row>|{right="changed";d.print(*(wide.view));d.print(wide.extra)}
    |wide<Text>|{left=42;d.print(*(wide.view))}
    left=43
    right="done"
    d.print(left)
    d.print(right)
}
inspect(true)
inspect(false)
x:5
s:"text"
<Small>:<&int32><&string>
a<Small>:&x
b<Small><null><boolean>:a
|b<Small>|{c<Small>:b;|c<&int32>|d.print(*(c~<&int32>))}
other<Small><null><boolean>:&s
d.print(b==other)
"#,
    )
    .runs(b"41\n2\n43\ndone\noriginal\n43\ndone\n5\nfalse\n");
}

#[test]
pub fn optional_reference_results_reset_when_control_discards_them() {
    Case::new(
        r#"
d:@"debug"
again:=true
value<{view<&int32><null>}>:'out {
    |again|{local:5;'out->view:&local;again=false;'out.restart()}
}
|value.view<null>|d.print("missing")
owner:=12
count:=0
'loop {
    item:{|count<2|->view:&owner}
    |item.view<&int32>|d.print(*(item.view~<&int32>))
    owner=owner+1
    count=count+1
    |count<3|'loop.restart()
}
d.print(owner)
present:=true
number:=20
step:=0
'choose {
    view<&int32><null>:{|present|->&number}
    |view<&int32>|d.print(*(view~<&int32>))
    |view<null>|number=21
    present=!present
    step=step+1
    |step<2|'choose.restart()
}
d.print(number)
"#,
    )
    .runs(b"missing\n12\n13\n15\n20\n21\n");
}

#[test]
pub fn reference_unions_protect_live_copies_operands_and_slots() {
    for source in [
        "a:=1;r<&int32><null>:&a;a=2;copy:r",
        "a:=1;flag:=true;r:{|flag|->view:&a};a=2;copy:r",
        "a:=1;r<&int32><null>:&a;same:r=={a=2;->null}",
        "a:=1;flag:=true;r:{|flag|->&a;a=2}",
        "a:=1;flag:=true;r:{|flag|->view:&a;a=2}",
        "a:=1;flag:=true;|({|flag|->&a;a=2})<null>|{}",
        "a:=1;r<&int32><null>:&a;i:=0;'loop{|r<&int32>|{value:*(r~<&int32>)};a=2;i=i+1;|i<2|'loop.restart()}",
        "a:=1;r<&int32><null>:&a;first:=true;i:=0;'loop{|!first&&r<&int32>|{value:*(r~<&int32>)};|first|a=2;first=false;i=i+1;|i<2|'loop.restart()}",
        "f<null>:(flag<boolean>){a:=1;r<&int32><null>:{|flag|->&a};|r<&int32>|{a=2;value:*(r~<&int32>)}}",
        "tag<int32><null>:=null;tag=1;copy:tag;a:=1;r:{|copy<int32>|->view:&a};a=2;|r.view<&int32>|{value:*(r.view~<&int32>)}",
        "tag<{value<int32><null>}>:={};tag={->value:1};copy:tag;a:=1;r:{|copy.value<int32>|->view:&a};a=2;|r.view<&int32>|{value:*(r.view~<&int32>)}",
    ] {
        let case = Case::new(source);
        for profile in ["debug", "release"] {
            let result = case.command("build", &["--json", "--profile", profile]);
            assert_eq!(result.status.code(), Some(1), "{source}");
            let error = String::from_utf8_lossy(&result.stderr);
            assert!(error.contains("\"code\":\"E302\""), "{source}: {error}");
            assert!(!case.path.join("build").exists());
        }
    }
}

#[test]
pub fn reference_union_escapes_and_remaining_contracts_are_explicit() {
    Case::new("a:1;r<&int32><null>:=&a").runs(b"");
    Case::new("f<null>:(r<&int32><null>){copy:=r}").runs(b"");
    for (source, code) in [
        ("flag:=true;r:{a:1;|flag|->&a}", "E303"),
        ("flag:=true;r:{a:1;|flag|->view:&a}", "E303"),
        ("flag:=true;r:{a:1;->nested:{|flag|->view:&a}}", "E303"),
        (
            "outer:1;flag:=true;r:{local:2;|flag|->&outer;|!flag|->&local}",
            "E303",
        ),
        ("a:1;r<&int32><null>:&a;d:@\"debug\";d.print(r)", "B001"),
    ] {
        let result = Case::new(source).command("check", &["--json"]);
        assert_eq!(result.status.code(), Some(1), "{source}");
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(
            error.contains(&format!("\"code\":\"{code}\"")),
            "{source}: {error}"
        );
    }
}

#[test]
pub fn nested_optional_tags_stay_conditioned_on_the_outer_variant() {
    Case::new(
        r#"
d:@"debug"
<A>:<{view<&int32><null>;count<int32>}>
<B>:<{view<&int32><null>;name<string>}>
inspect<null>:(flag<boolean>){
    owner:=21
    present<A>:{->view:&owner;->count:1}
    absent<B>:{->name:"empty"}
    value<A><B>:{|flag|->present;|!flag|->absent}
    |value<B>|{owner=22;d.print(value.name);|value.view<null>|d.print("missing")}
    |value<A>|{|value.view<&int32>|d.print(*(value.view~<&int32>))}
    owner=23
    d.print(owner)
}
inspect(true)
inspect(false)
"#,
    )
    .runs(b"21\n23\nempty\nmissing\n23\n");
    let source = r#"
<A>:<{view<&int32><null>;count<int32>}>
<B>:<{view<&int32><null>;name<string>}>
f<null>:(flag<boolean>){
    owner:=21
    present<A>:{->view:&owner;->count:1}
    absent<B>:{->name:"empty"}
    value<A><B>:{|flag|->present;|!flag|->absent}
    |value<A>|{owner=22;|value.view<&int32>|{read:*(value.view~<&int32>)}}
}
"#;
    let case = Case::new(source);
    for profile in ["debug", "release"] {
        let result = case.command("build", &["--json", "--profile", profile]);
        assert_eq!(result.status.code(), Some(1));
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(error.contains("\"code\":\"E302\""), "{error}");
    }
}
