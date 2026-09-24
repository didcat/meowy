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
pub fn restarted_reference_cells_preserve_nested_targets_and_old_copies() {
    Case::new(
        r#"
d:@"debug"
a:7;b:9;left:&a;right:&b;p:=&left;old:*p;count:=0
'loop{d.print(**p);p=&right;count=count+1;|count<2|'loop.restart()}
d.print(*old);d.print(**p)
"#,
    )
    .runs(b"7\n9\n7\n9\n");
}

#[test]
pub fn restarted_carrier_fields_keep_distinct_component_sources() {
    Case::new(
        r#"
d:@"debug"
a:1;b:2;c:3
left:{->first:&a;->second:&b;->count:10}
right:{->first:&b;->second:&c;->count:20}
p:=&left;saved:*p;count:=0
'loop{
    d.print(*(p.first));d.print(*(p.second));d.print(p.count)
    p=&right
    count=count+1
    |count<2|'loop.restart()
}
d.print(*(saved.first));d.print(*(saved.second))
"#,
    )
    .runs(b"1\n2\n10\n2\n3\n20\n1\n2\n");
}

#[test]
pub fn restart_pointer_and_scalar_reads_do_not_consume_pointee_references() {
    Case::new(
        r#"
d:@"debug"
a:=1;b:=2
left:{->view:&a;->count:7};right:{->view:&b;->count:8}
p:=&left;count:=0
'loop{
    a=3;b=4
    d.print(p==&left);d.print(p.count)
    p=&right
    count=count+1
    |count<2|'loop.restart()
}
"#,
    )
    .runs(b"true\n7\nfalse\n8\n");
    for source in [
        "a:=1;b:2;left:{->view:&a};right:{->view:&b};p:=&left;i:=0;'loop{a=3;v:*(p.view);p=&right;i=i+1;|i<2|'loop.restart()}",
        "a:1;b:=2;left:{->view:&a};right:{->view:&b};p:=&left;i:=0;'loop{b=3;v:*(p.view);p=&right;i=i+1;|i<2|'loop.restart()}",
        "a:=1;b:2;left:{->view:&a};right:{->view:&b};p:=&left;old:p;i:=0;'loop{p=&right;i=i+1;|i<2|'loop.restart()};a=3;v:*(old.view)",
    ] {
        rejects(source, "E302");
    }
}

#[test]
pub fn transitive_restart_copies_preserve_public_bounds_at_each_reference_layer() {
    Case::new(
        r#"
d:@"debug"
<H>:<{view<&int32>}>
first<&H>:(p<&H>,text<&string>){->p}
a:1;b:2;left<H>:{->view:&a};right<H>:{->view:&b}
text:="old";p:=first(&left,&text);count:=0
'loop{copy:*p;d.print(*(copy.view));p=&right;count=count+1;|count<2|'loop.restart()}
text="new"
d.print(text)
"#,
    )
    .runs(b"1\n2\nnew\n");
    for source in [
        "<H>:<{view<&int32>}>;first<&H>:(p<&H>,text<&string>){->p};a:1;b:2;left<H>:{->view:&a};right<H>:{->view:&b};text:=\"old\";p:=first(&left,&text);copy:*p;i:=0;'loop{p=&right;i=i+1;|i<2|'loop.restart()};text=\"new\";v:*(copy.view)",
        "<R>:<&int32>;inner<R>:(p<R>,s<&string>){->p};outer<&R>:(p<&R>,s<&string>){->p};a:1;text:=\"old\";q:=inner(&a,&text);p:=&q;i:=0;'loop{same:p==&q;q=&a;p=outer(&q,&text);text=\"new\";i=i+1;|i<2|'loop.restart()}",
    ] {
        rejects(source, "E302");
    }
}

#[test]
pub fn nested_references_to_reference_free_unions_need_no_header_activity() {
    Case::new(
        r#"
d:@"debug"
left<int32><null>:7;right<int32><null>:null
first:&left;second:&right;p:=&first;count:=0
'loop{
    copy:**p
    |copy<int32>|d.print(copy~<int32>)
    |copy<null>|d.print("empty")
    p=&second
    count=count+1
    |count<2|'loop.restart()
}
"#,
    )
    .runs(b"7\nempty\n");
    for source in [
        "<H>:<{view<&int32><null>}>;a:1;holder<H>:{->view:&a};p:=&holder;i:=0;'loop{p=&holder;i=i+1;|i<2|'loop.restart()}",
        "<H>:<{view<&int32>;tag<int32><null>}>;a:1;holder<H>:{->view:&a;->tag:1};p:=&holder;i:=0;'loop{p=&holder;i=i+1;|i<2|'loop.restart()}",
        "<A>:<{view<&int32>}>;<B>:<{other<&int32>}>;a:1;holder<A><B>:{->view:&a};p:=&holder;i:=0;'loop{p=&holder;i=i+1;|i<2|'loop.restart()}",
    ] {
        let output = Case::new(source).command("check", &["--json"]);
        assert!(
            output.status.success(),
            "{source}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
    }
}

#[test]
pub fn transitive_header_rotation_preserves_projected_paths_and_nested_targets() {
    Case::new(
        r#"
d:@"debug"
a:1;b:2
left:{->first:&a;->second:&b}
right:{->first:&b;->second:&a}
box:{->left:&left;->right:&right}
p:=box.left;q:=box.right;count:=0
'outer{
    inner:=0
    'inner{
        d.print(*(p.first));d.print(*(p.second))
        old:p;p=q;q=old
        inner=inner+1
        |inner<2|'inner.restart()
    }
    old:p;p=q;q=old
    count=count+1
    |count<2|'outer.restart()
}
"#,
    )
    .runs(b"1\n2\n2\n1\n2\n1\n1\n2\n");
}

#[test]
pub fn restarted_outer_cell_views_keep_physical_and_contained_loans_separate() {
    Case::new(
        r#"
d:@"debug"
a:1;b:2;cell:=&a;p:=&cell;count:=0
'loop{
    d.print(**p)
    cell=&b
    p=&cell
    count=count+1
    |count<2|'loop.restart()
}
"#,
    )
    .runs(b"1\n2\n");
    for source in [
        "a:1;b:2;cell:=&a;p:=&cell;i:=0;'loop{cell=&b;v:**p;p=&cell;i=i+1;|i<2|'loop.restart()}",
        "a:1;b:2;left:&a;right:&b;p:=&left;outer:&p;i:=0;'loop{p=&right;i=i+1;|i<2|'loop.restart()};v:***outer",
    ] {
        rejects(source, "E302");
    }
}

#[test]
pub fn nested_headers_allow_unread_expiry_but_reject_expired_values() {
    for source in [
        "cell:&1;p:=&cell;i:=0;'loop{p=&cell;i=i+1;|i<2|'loop.restart()}",
        "first<&int32>:(p<&int32>,text<&string>){->p};a:1;cell:first(&a,&\"short\");p:=&cell;i:=0;'loop{p=&cell;i=i+1;|i<2|'loop.restart()}",
        "a:1;initial:&a;p:=&initial;i:=0;'loop{local:2;cell:&local;p=&cell;i=i+1;|i<2|'loop.restart()}",
    ] {
        Case::new(source).runs(b"");
    }
    rejects(
        "holder:{->view:&1};p:=&holder;i:=0;'loop{p=&holder;i=i+1;|i<2|'loop.restart()}",
        "E303",
    );
    Case::new(
        r#"
d:@"debug"
a:7;cell:&a;p:=&cell;count:=0
'loop{
    local:9;inside:&local
    p=&inside
    d.print(**p)
    p=&cell
    count=count+1
    |count<2|'loop.restart()
}
"#,
    )
    .runs(b"9\n9\n");
}
