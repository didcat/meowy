use super::Case;

#[test]
pub fn emitted_reference_values_keep_pointee_identity_after_slot_exit() {
    Case::new(
        r#"
d:@"debug"
forward<&int32>:(input<&int32>){holder:{->view:input};->holder.view}
owner:=7
copy:{->view:&owner;->view}
d.print(copy.view==&owner);d.print(*(copy.view))
returned:forward(&owner)
d.print(returned==&owner);d.print(*returned)
projected:{holder:{->view:&owner;->count:3};->holder.count}
owner=8
d.print(projected);d.print(owner)
"#,
    )
    .runs(b"true\n7\ntrue\n7\n3\n8\n");
}

#[test]
pub fn emitted_reference_carriers_support_selected_storage_and_pointee_views() {
    Case::new(
        r#"
d:@"debug"
owner:={->n:7}
value:{
    ->holder:{->view:&owner;->count:4;->items<int32[3]>:[5,6]}
    count:&(holder.count)
    d.print(count==&(holder.count));d.print(*count)
    copy:holder
    d.print(count==&(copy.count))
    nested:&(holder.view.n)
    d.print(nested==&(owner.n));d.print(*nested)
    item:&(holder.items[2])
    d.print(*item)
}
d.print(value.holder.count)
value.holder.{p:&($.count);d.print(*p)}
<Carrier>:<{view<&{n<int32>}>;count<int32>;items<int32[3]>}>
show<null>:(input<Carrier>){p:&(input.count);d.print(*p)}
show(value.holder)
owner={->n:8}
d.print(value.holder.count)
"#,
    )
    .runs(b"true\n4\nfalse\ntrue\n7\n6\n4\n4\n4\n4\n");
}

#[test]
pub fn carrier_field_borrows_follow_cell_owners_without_unrelated_pointee_loans() {
    Case::new(
        r#"
d:@"debug"
owner:=1
holder:{->view:&owner;->count:2}
count:&(holder.count)
owner=3
d.print(*count)
value:'out{
    field:{'out->holder:{->view:&owner;->count:4};->&(holder.count)}
    d.print(*field)
}
d.print(value.holder.count)
'escape{
    value:'target{
        field:{local:5;'target->holder:{->view:&local;->count:6};->&(holder.count)}
        d.print(*field)
        'escape.leave()
    }
}
"#,
    )
    .runs(b"2\n4\n4\n6\n");
}

#[test]
pub fn reference_slot_widening_preserves_active_origins_and_selected_payloads() {
    Case::new(
        r#"
d:@"debug"
choose:(flag<boolean>,input<&int32>)'out{
    |flag|{'out->view:input;d.print(*view)}
    |!flag|{'out->view:"absent";d.print(view)}
}
owner:=7
one:choose(true,&owner).view
two:choose(false,&owner).view
|one<&int32>|d.print(*(one<&int32>))
|two<string>|d.print(two<string>)
maybe:(flag<boolean>,input<&int32>)'out{
    |flag|{'out->holder:{->view:input;->count:8};p:&(holder.count);d.print(*p)}
}
present:maybe(true,&owner).holder
absent:maybe(false,&owner).holder
|present<{view<&int32>;count<int32>}>|d.print(*(present.view))
|absent<null>|d.print("null")
owner=9
d.print(owner)
"#,
    )
    .runs(b"7\nabsent\n7\nabsent\n8\n7\nnull\n9\n");
}

#[test]
pub fn reference_slots_preserve_restart_and_guarded_carrier_facts() {
    Case::new(
        r#"
d:@"debug"
owner:=1
count:=0
value:'again{
    ->holder:{->view:&owner;->count:count}
    p:&(holder.count)
    d.print(*p)
    count=count+1
    |count<2|'again.restart()
}
d.print(*(value.holder.view))
outer:{->holder:{->view:&owner;->count:3};p:&(holder.count);i:=0;'loop{d.print(*p);i=i+1;|i<2|'loop.restart()}}
empty:{->holder:{->view<&int32><null>:null;->count:4};p:&(holder.count);d.print(*p)}
owner=2
|empty.holder.view<null>|d.print("empty")
d.print(owner)
"#,
    )
    .runs(b"0\n1\n1\n3\n3\n4\nempty\n2\n");
}

#[test]
pub fn reference_slot_views_preserve_write_and_publication_rejections() {
    for (source, code) in [
        (
            "owner:=1;value:{->view:&owner};owner=2;copy:*(value.view)",
            "E302",
        ),
        (
            "owner:=1;value:{->holder:{->view:&owner;->count:3}};owner=2;copy:*(value.holder.view)",
            "E302",
        ),
        (
            "owner:=1;value:{->holder:{->view:&owner;->count:3};p:&(holder.count);owner=2;copy:*p}",
            "E302",
        ),
        (
            "owner:1;value:{->holder:{->view:&owner;->count:3};->field:&(holder.count)}",
            "E303",
        ),
        (
            "owner:1;value:'out{field:{'out->holder:{->view:&owner;->count:3};->&(holder.count)};->saved:field}",
            "E303",
        ),
        ("value:{local:1;->view:&local}", "E303"),
        ("value:{local:1;->holder:{->view:&local;->count:3}}", "E303"),
        (
            "value:{owner:1;holder:{->view:&owner;->count:3};->&(holder.count)}",
            "E303",
        ),
        (
            "bad<&int32>:(input<{view<&int32>;count<int32>}>){->&(input.count)}",
            "E303",
        ),
        (
            "owner:1;holder:{->view:&owner;->count:3};p:holder.{->&($.count)}",
            "E303",
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
pub fn emitted_reference_calls_keep_all_input_bounds_and_safe_copy_escapes() {
    Case::new(
        r#"
d:@"debug"
pick<&int32>:(input<&int32>,other<&string>){holder:{->view:input};->holder.view}
owner:=1
other:="before"
view:pick(&owner,&other)
d.print(*view)
owner=2;other="after"
d.print(owner);d.print(other)
"#,
    )
    .runs(b"1\n2\nafter\n");
    for (source, code) in [
        (
            "pick<&int32>:(input<&int32>,other<&string>){holder:{->view:input};->holder.view};owner:=1;other:=\"before\";view:pick(&owner,&other);other=\"after\";copy:*view",
            "E302",
        ),
        (
            "pick<&int32>:(input<&int32>,other<&string>){holder:{->view:input};->holder.view};owner:1;view:{other:\"local\";->pick(&owner,&other)}",
            "E303",
        ),
    ] {
        let output = Case::new(source).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains(&format!("\"code\":\"{code}\"")));
    }
}

#[test]
pub fn reference_slot_storage_keeps_whole_carrier_and_mutation_boundaries() {
    for source in [
        "owner:1;n:=2;value:'out{'loop{'out->view:=&owner;view=&owner;n=n-1;|n>0|'loop.restart()}}",
        "owner:1;n:=2;value:'out{'loop{'out->holder:={->view:&owner;->count:3};holder={->view:&owner;->count:4};n=n-1;|n>0|'loop.restart()}}",
        "owner:1;value:{->holder:{->view:&owner;->count:3};address:&!(holder.count)}",
    ] {
        let output = Case::new(source).command("build", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"B001\""), "{source}: {error}");
    }
}

#[test]
pub fn reference_carrier_list_fields_keep_checked_bounds_and_index_effects() {
    let source = "d:@\"debug\";owner:1;value:{->holder:{->view:&owner;->items<int32[3]>:[1]};index:=2;p:&(holder.items[{d.print(\"index\");->index}])}";
    let access = "&(holder.items[{d.print(\"index\");->index}])";
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
