use super::Case;

#[test]
pub fn immutable_emitted_borrows_preserve_identity_and_once_only_initialization() {
    Case::new(
        r#"
d:@"debug"
initial<int32>:(){d.print("init");->7}
value:{
    ->n:initial()
    copy:n
    view:&n
    d.print(view==&n);d.print(view==&copy);d.print(*view)
    ->enabled:true
    |enabled|d.print("enabled")
}
d.print(value.n)
"#,
    )
    .runs(b"init\ntrue\nfalse\n7\nenabled\n7\n");
}

#[test]
pub fn immutable_emitted_fields_and_elements_keep_original_storage() {
    Case::new(
        r#"
d:@"debug"
first<&int32>:(items<&int32[3]>){->&(items[1])}
value:{
    ->child:{->n:=1;->name:"row"}
    view:&(child.n)
    copy:=child
    copy.n=2
    d.print(*view);d.print(copy.n)
    parent:&child
    d.print(view==&(parent.n))
    ->items<int32[3]>:[3,4]
    item:first(&items)
    d.print(item==&(items[1]));d.print(*item)
    other:=items
    other[1]=5
    d.print(*item);d.print(other[1])
}
d.print(value.child.n);d.print(value.items[2])
"#,
    )
    .runs(b"1\n2\ntrue\ntrue\n3\n3\n5\n1\n4\n");
}

#[test]
pub fn immutable_emitted_borrows_follow_named_and_discarded_target_lifetimes() {
    Case::new(
        r#"
d:@"debug"
value:'out{view:{'out->n:6;->&n};d.print(*view);->seen:*view}
d.print(value.n);d.print(value.seen)
run<null>:(flag<boolean>){
    'escape{
        value:'target{
            |flag|{view:{'target->lost:7;->&lost};d.print(*view);'escape.leave()}
            ->"kept"
        }
        d.print(value)
    }
}
run(true);run(false)
opposite<null>:(flag<boolean>){
    'escape{
        value:'target{
            |flag|{view:{'target->n:8;->&n};d.print(*view);'escape.leave()}
            ->n:=9
        }
        d.print(value.n)
    }
}
opposite(true);opposite(false)
"#,
    )
    .runs(b"6\n6\n6\n7\nkept\n8\n9\n");
}

#[test]
pub fn immutable_emitted_borrows_preserve_widened_payloads_and_restart() {
    Case::new(
        r#"
d:@"debug"
choose:(flag<boolean>)'out{
    |flag|{'out->n:1;view:&n;d.print(*view)}
    |!flag|{'out->n:"text";view:&n;d.print(*view)}
}
one:choose(true).n
two:choose(false).n
|one<int32>|d.print(one~<int32>)
|two<string>|d.print(two~<string>)
maybe:(flag<boolean>)'out{|flag|{'out->row:{->n:2};view:&(row.n);d.print(*view)}}
present:maybe(true).row
absent:maybe(false).row
|present<{n<int32>}>|d.print(present.n)
|absent<null>|d.print("absent")
count:=0
value:'again{->n:count;view:&n;d.print(*view);count=count+1;|count<2|'again.restart()}
d.print(value.n)
outer:{->n:9;view:&n;i:=0;'loop{d.print(*view);i=i+1;|i<2|'loop.restart()}}
d.print(outer.n)
"#,
    )
    .runs(b"1\ntext\n1\ntext\n2\n2\nabsent\n0\n1\n1\n9\n9\n9\n");
}

#[test]
pub fn immutable_emitted_aliases_preserve_variant_facts_and_reference_carriers() {
    Case::new(
        r#"
d:@"debug"
owner:=1
value:{
    ->tag<int32><null>:null
    ->view<&int32><null>:{|tag<int32>|->&owner}
    |tag<null>|d.print("null")
}
owner=2
|value.view<null>|d.print("empty")
d.print(owner)
kept:{->view:&owner;copy:view;d.print(*copy);->n:3;r:&n;d.print(*r)}
d.print(*(kept.view))
owner=4
d.print(owner)
"#,
    )
    .runs(b"null\nempty\n2\n2\n3\n2\n4\n");
}

#[test]
pub fn immutable_emitted_aliases_reject_writes_and_escaping_borrows() {
    Case::new("value:{->child:{->n:=1};child.n=2}").runs(b"");
    Case::new("value:{->items:[{->n:=1}];items[1].n=2}").runs(b"");
    for (source, code) in [
        ("value:{->n:1;n=2}", "E305"),
        ("value:{->items:[1];items[1]=2}", "E305"),
        ("value:{->n:1;->view:&n}", "E303"),
        ("value:'out{view:{'out->n:1;->&n};->saved:view}", "E303"),
        ("value:{->items:[1];->view:&(items[1])}", "E303"),
        ("bad<{n<int32>;view<&int32>}>:(){->n:1;->view:&n}", "E303"),
        (
            "owner:=1;value:{->view:&owner};owner=2;copy:*(value.view)",
            "E302",
        ),
        ("value:{->n:1;->n:=2}", "E205"),
        ("value<{n<int32>}>:{->n:=1}", "E206"),
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
pub fn immutable_emitted_lists_keep_static_and_dynamic_bounds() {
    for source in [
        "value:{->items<int32[3]>:[1];view:&(items[2])}",
        "value:{->items<int32[3]>:[1];copy:items[2]}",
    ] {
        let output = Case::new(source).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E101\""));
    }
    let source = "d:@\"debug\";value:{->items<int32[3]>:[1];index:=2;view:&(items[{d.print(\"index\");->index}])}";
    let access = "&(items[{d.print(\"index\");->index}])";
    let start = source.find(access).unwrap();
    let end = start + access.len();
    let case = Case::new(source);
    for profile in ["debug", "release"] {
        let output = case.command("run", &["--profile", profile]);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, b"index\n");
        assert_eq!(
            output.stderr,
            format!(
                "panic[P001]: index 2 is outside initialized length 1 at bytes {start}..{end}\n"
            )
            .as_bytes()
        );
    }
}

#[test]
pub fn immutable_emitted_borrows_keep_unrepresented_storage_explicit() {
    super::exclusive_references::rejects("value:{->n:1;view:&!n}", "E305");
    Case::new("value:{->row:{->n:=1};view:&!(row.n)}").runs(b"");
    for source in [
        "value:{->row:{->n:=1};view:&!row}",
        "choose:(flag<boolean>)'out{|flag|{'out->n<int32><null>:null;view:&n};|!flag|{'out->n:\"text\"}}",
    ] {
        let case = Case::new(source);
        for profile in ["debug", "release"] {
            let output = case.command("build", &["--json", "--profile", profile]);
            assert_eq!(output.status.code(), Some(1), "{source}");
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains("\"code\":\"B001\""), "{source}: {error}");
            assert!(!case.path.join("build").exists());
        }
    }
}
