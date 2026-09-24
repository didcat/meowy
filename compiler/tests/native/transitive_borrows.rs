use super::Case;

#[test]
pub fn nested_references_and_carrier_borrows_preserve_pointer_identity() {
    Case::new(
        r#"
d:@"debug"
owner:=7
cell:&owner
outer:&cell
third:&outer
d.print(*outer==&owner);d.print(**third==cell);d.print(***third)
holder:{->view:&owner;->n:3}
view:&holder
slot:&(view.view)
d.print(slot==&(holder.view));d.print(**slot)
copy:*view
d.print(*(copy.view));d.print(view.n)
owner=8
d.print(owner)
"#,
    )
    .runs(b"true\ntrue\n7\ntrue\n7\n7\n3\n8\n");
}

#[test]
pub fn direct_dereference_copies_outlive_cells_but_keep_contained_owners() {
    Case::new(
        r#"
d:@"debug"
owner:7
copy:{holder:{->view:&owner;->n:3};outer:&holder;->*outer}
d.print(*(copy.view));d.print(copy.n)
inner:{cell:&owner;outer:&cell;->*outer}
d.print(inner==&owner)
result:{->cell:&owner;outer:&cell;->*outer}
d.print(*(result.cell))
row:{->n:9}
field:{holder:{->view:&row};outer:&holder;->&(outer.view.n)}
d.print(field==&(row.n));d.print(*field)
"#,
    )
    .runs(b"7\n3\ntrue\n7\ntrue\n9\n");
}

#[test]
pub fn nested_projection_and_tag_reads_do_not_consume_unrelated_pointees() {
    Case::new(
        r#"
d:@"debug"
left:=1
right:=2
holder:{->left:&left;->right:&right;->n:3}
outer:&holder
right=4
selected:outer.left
d.print(*selected);d.print(outer.n);d.print(outer==&holder)
left=5
d.print(right);d.print(left)
tagged:{->view<&int32><null>:&left}
tag:&tagged
left=6
|tag.view<&int32>|d.print("reference")
d.print(left)
"#,
    )
    .runs(b"1\n3\ntrue\n4\n5\nreference\n6\n");
}

#[test]
pub fn nested_reference_contracts_preserve_contents_and_all_input_bounds() {
    Case::new(
        r#"
d:@"debug"
<H>:<{view<&int32>;n<int32>}>
<R>:<&int32>
load<R>:(cell<&R>){->*cell}
copy<H>:(holder<&H>){->*holder}
field<&int32>:(holder<&H>){->&(holder.n)}
value<&int32>:(holder<&H>){->holder.view}
by_value<H>:(holder<H>){borrowed:&holder;->*borrowed}
owner:7
cell:&owner
d.print(*(load(&cell)))
holder<H>:{->view:&owner;->n:3}
cloned:copy(&holder)
d.print(*(cloned.view));d.print(*(field(&holder)));d.print(*(value(&holder)))
escaped:{local<H>:{->view:&owner;->n:9};->by_value(local)}
d.print(*(escaped.view));d.print(escaped.n)
"#,
    )
    .runs(b"7\n7\n3\n7\n7\n9\n");
}

#[test]
pub fn nested_nullable_snapshots_and_call_candidates_keep_variant_correlations() {
    Case::new(
        r#"
d:@"debug"
<H>:<{view<&int32><null>;n<int32>}>
pick<&H>:(flag<boolean>,left<&H>,right<&H>){|flag|->left;|!flag|->right}
owner:7
empty<H>:{->view:null;->n:1}
full<H>:{->view:&owner;->n:2}
first:pick(true,&empty,&full)
second:pick(false,&empty,&full)
one:*first
two:*second
|one.view<null>|d.print("empty")
|two.view<&int32>|d.print(*(two.view~<&int32>))
d.print(first.n);d.print(second.n)
cell:&(empty.view)
copied:*cell
|copied<null>|d.print("null")
"#,
    )
    .runs(b"empty\n7\n1\n2\nnull\n");
}

#[test]
pub fn transitive_loans_reject_stale_nested_reads_and_invalid_escapes() {
    for (source, code) in [
        (
            "owner:=1;holder:{->view:&owner};owner=2;outer:&holder;v:*(outer.view)",
            "E302",
        ),
        ("owner:=1;cell:&owner;outer:&cell;owner=2;v:**outer", "E302"),
        (
            "a:=1;b:=2;holder:{->left:&a;->right:&b};outer:&holder;b=3;copy:*outer",
            "E302",
        ),
        ("owner:1;copy:{cell:&owner;->&cell}", "E303"),
        (
            "copy:{owner:1;holder:{->view:&owner};outer:&holder;->*outer}",
            "E303",
        ),
        (
            "owner:1;result:{->holder:{->view:&owner};->outer:&holder}",
            "E303",
        ),
        (
            "<H>:<{view<&int32>}>;copy<H>:(p<&H>){->*p};owner:1;result:{holder<H>:{->view:&owner};->copy(&holder)}",
            "E303",
        ),
        (
            "<R>:<&int32>;load<R>:(p<&R>,other<&string>){->*p};owner:1;cell:&owner;other:=\"a\";view:load(&cell,&other);other=\"b\";v:*view",
            "E302",
        ),
        (
            "<H>:<{view<&int32>;n<int32>}>;bad<&int32>:(p<H>){->&(p.n)}",
            "E303",
        ),
        (
            "owner:=1;tag<int32><null>:=null;tag=1;view:&tag;copy:*view;result<&int32><null>:{|copy<int32>|->&owner};owner=2;|result<&int32>|v:*(result~<&int32>)",
            "E302",
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
pub fn transitive_borrows_reset_iteration_summaries_and_keep_outer_loans() {
    Case::new(
        r#"
d:@"debug"
owner:=1
i:=0
'again{
    holder:{->view:&owner}
    outer:&holder
    d.print(*(outer.view))
    owner=owner+1
    i=i+1
    |i<2|'again.restart()
}
left:7
right:9
count:=0
result:'target{
    first:count==0
    ->cell<&int32>:{|first|->&left;|!first|->&right}
    outer:&cell
    d.print(**outer)
    count=count+1
    |count<2|'target.restart()
}
d.print(*(result.cell))
"#,
    )
    .runs(b"1\n2\n7\n9\n9\n");
    let output = Case::new(
        "owner:=1;cell:&owner;outer:&cell;i:=0;'loop{v:**outer;owner=2;i=i+1;|i<2|'loop.restart()}",
    )
    .command("check", &["--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E302\""));
}

#[test]
pub fn transitive_borrow_boundaries_stay_explicit() {
    Case::new("owner:1;holder:={->view:&owner}").runs(b"");
    Case::new("owner:1;cell<&int32><null>:=&owner").runs(b"");
    for source in [
        "owner:1;items:[&owner]",
        "owner:1;n:=2;holder:'out{'loop{'out->view:=&owner;view=&owner;n=n-1;|n>0|'loop.restart()}}",
    ] {
        let output = Case::new(source).command("build", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"B001\""));
    }
}
