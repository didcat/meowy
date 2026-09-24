use super::Case;

pub(crate) fn rejects(source: &str, code: &str) {
    let output = Case::new(source).command("check", &["--json"]);
    assert_eq!(output.status.code(), Some(1), "{source}");
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error
            .lines()
            .next()
            .unwrap_or_default()
            .contains(&format!("\"code\":\"{code}\"")),
        "{source}: {error}"
    );
}

#[test]
pub fn restart_activity_carries_null_full_and_null_again() {
    Case::new(
        r#"
d:@"debug"
<H>:<{view<&int32><null>;count<int32>}>
a:7;empty<H>:{->count:1};full<H>:{->view:&a;->count:2}
p:=&empty;i:=0
'loop{
    copy:*p
    |copy.view<null>|d.print("empty")
    |copy.view<&int32>|d.print(*(copy.view~<&int32>))
    d.print(copy.count)
    i=i+1
    |i==1|p=&full
    |i==2|p=&empty
    |i<3|'loop.restart()
}
"#,
    )
    .runs(b"empty\n1\n7\n2\nempty\n1\n");
}

#[test]
pub fn inactive_backedges_end_finished_payload_loans_without_new_reads() {
    Case::new(
        r#"
d:@"debug"
<H>:<{view<&int32><null>;count<int32>}>
owner:=7;empty<H>:{->count:1};full<H>:{->view:&owner;->count:2}
p:=&full;i:=0
'loop{
    copy:*p
    |copy.view<&int32>|d.print(*(copy.view~<&int32>))
    p=&empty
    owner=8
    i=i+1
    |i<2|'loop.restart()
}
d.print(owner)
p=&empty;i=0
'empty{
    owner=9
    copy:*p
    |copy.view<null>|d.print(copy.count)
    p=&empty
    i=i+1
    |i<2|'empty.restart()
}
"#,
    )
    .runs(b"7\n8\n1\n1\n");
}

#[test]
pub fn restart_activity_resets_cannot_hide_initial_or_future_payload_conflicts() {
    for source in [
        "<H>:<{view<&int32><null>}>;a:=1;empty<H>:{};full<H>:{->view:&a};p:=&empty;i:=0;'loop{copy:*p;|copy.view<null>|a=2;|copy.view<&int32>|v:*(copy.view~<&int32>);p=&full;i=i+1;|i<2|'loop.restart()}",
        "<H>:<{view<&int32><null>}>;a:=1;empty<H>:{};full<H>:{->view:&a};p:=&full;i:=0;'loop{a=2;copy:*p;|copy.view<&int32>|v:*(copy.view~<&int32>);p=&empty;i=i+1;|i<2|'loop.restart()}",
        "<H>:<{view<&int32><null>}>;a:1;empty<H>:{};full<H>:{->view:&a};p:=&empty;cell:&p;i:=0;'loop{p=&full;i=i+1;|i<2|'loop.restart()};copy:**cell",
    ] {
        rejects(source, "E302");
    }
}

#[test]
pub fn old_header_copies_keep_their_original_activity() {
    Case::new(
        r#"
d:@"debug"
<H>:<{view<&int32><null>;count<int32>}>
a:=7;empty<H>:{->count:1};full<H>:{->view:&a;->count:2}
p:=&empty;old:*p;p=&full;full_copy:*p;i:=0
'loop{p=&empty;i=i+1;|i<2|'loop.restart()}
|old.view<null>|d.print("old-empty")
|full_copy.view<&int32>|d.print(*(full_copy.view~<&int32>))
a=8
|full_copy.view<&int32>|d.print("tag-only")
"#,
    )
    .runs(b"old-empty\n7\ntag-only\n");
    rejects(
        "<H>:<{view<&int32><null>}>;a:=1;empty<H>:{};full<H>:{->view:&a};p:=&full;old:*p;i:=0;'loop{p=&empty;i=i+1;|i<2|'loop.restart()};a=2;|old.view<&int32>|v:*(old.view~<&int32>)",
        "E302",
    );
}

#[test]
pub fn nested_header_activity_conditions_children_on_their_parent_variant() {
    Case::new(
        r#"
d:@"debug"
<A>:<{view<&int32><null>;n<int32>}>
<B>:<{other<&int32>;n<int32>}>
a:7;b:9
none<A><B>:{->n:1}
some<A><B>:{->view:&a;->n:2}
other<A><B>:{->other:&b;->n:3}
p:=&none;i:=0
'loop{
    copy:*p
    |copy<A>|{
        active:copy~<A>
        |active.view<null>|d.print("none")
        |active.view<&int32>|d.print(*(active.view~<&int32>))
        d.print(active.n)
    }
    |copy<B>|{active:copy~<B>;d.print(*(active.other));d.print(active.n)}
    i=i+1
    |i==1|p=&some
    |i==2|p=&other
    |i<3|'loop.restart()
}
"#,
    )
    .runs(b"none\n1\n7\n2\n9\n3\n");
}

#[test]
pub fn optional_record_fields_retain_their_own_paths_during_restart_rotation() {
    Case::new(
        r#"
d:@"debug"
<H>:<{left<&int32><null>;right<&int32><null>}>
a:1;b:2;left<H>:{->left:&a};right<H>:{->right:&b}
p:=&left;q:=&right;i:=0
'loop{
    copy:*p
    |copy.left<&int32>|d.print(*(copy.left~<&int32>))
    |copy.right<&int32>|d.print(*(copy.right~<&int32>))
    old:p;p=q;q=old
    i=i+1
    |i<3|'loop.restart()
}
"#,
    )
    .runs(b"1\n2\n1\n");
}

#[test]
pub fn inactive_header_leaves_have_no_public_bound_but_active_copies_keep_it() {
    Case::new(
        r#"
d:@"debug"
<H>:<{view<&int32><null>;n<int32>}>
empty<H>:(text<&string>){->n:1}
text:="old";none:empty(&text);p:=&none;i:=0
'loop{
    text="new"
    copy:*p
    |copy.view<null>|d.print(copy.n)
    p=&none
    i=i+1
    |i<2|'loop.restart()
}
"#,
    )
    .runs(b"1\n1\n");
    rejects(
        "<H>:<{view<&int32><null>}>;make<H>:(p<&int32>,text<&string>){->view:p};a:1;text:=\"old\";full:make(&a,&text);empty<H>:{};p:=&full;copy:*p;i:=0;'loop{p=&empty;i=i+1;|i<2|'loop.restart()};text=\"new\";|copy.view<&int32>|v:*(copy.view~<&int32>)",
        "E302",
    );
}

#[test]
pub fn activity_replay_keeps_call_effects_once_and_unread_expiry() {
    Case::new(
        r#"
d:@"debug"
<H>:<{view<&int32><null>;n<int32>}>
identity<&H>:(p<&H>){d.print("call");->p}
a:7;empty<H>:{->n:1};full<H>:{->view:&a;->n:2};p:=&empty;i:=0
'loop{
    copy:*p
    d.print(copy.n)
    p=identity(&full)
    i=i+1
    |i<2|'loop.restart()
}
"#,
    )
    .runs(b"1\ncall\n2\ncall\n");
    for source in [
        "<H>:<{view<&int32><null>}>;a:1;empty<H>:{};p:=&empty;i:=0;'loop{local:2;full<H>:{->view:&local};p=&full;i=i+1;|i<2|'loop.restart()}",
        "<H>:<{view<&int32><null>}>;first<&H>:(p<&H>,text<&string>){->p};a:1;full<H>:{->view:&a};p:=first(&full,&\"short\");i:=0;'loop{p=&full;i=i+1;|i<2|'loop.restart()}",
    ] {
        Case::new(source).runs(b"");
    }
    Case::new("a:1;p<&int32><null>:=&a").runs(b"");
    rejects("a:1;items:[&a]", "B001");
}
