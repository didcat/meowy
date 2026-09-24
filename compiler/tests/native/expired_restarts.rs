use super::Case;

pub(crate) fn rejects(source: &str) {
    let output = Case::new(source).command("check", &["--json"]);
    assert_eq!(output.status.code(), Some(1), "{source}");
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error
            .lines()
            .next()
            .unwrap_or_default()
            .contains("\"code\":\"E303\""),
        "{source}: {error}"
    );
}

#[test]
pub fn expired_entry_values_can_be_overwritten_before_any_read() {
    for setup in [
        "p:=&a;'owner{local:2;p=&local}",
        "p:=&a;'owner{->n:2;p=&n}",
        "p:=&2",
        "first<&int32>:(p<&int32>,text<&string>){->p};p:=first(&a,&\"short\")",
    ] {
        Case::new(&format!(
            "d:@\"debug\";a:7;{setup};i:=0;'loop{{p=&a;d.print(*p);i=i+1;|i<2|'loop.restart()}}"
        ))
        .runs(b"7\n7\n");
        rejects(&format!(
            "a:7;{setup};i:=0;'loop{{v:*p;p=&a;i=i+1;|i<2|'loop.restart()}}"
        ));
    }
}

#[test]
pub fn expired_backedge_values_can_be_overwritten_before_any_read() {
    for update in [
        "local:i;p=&local",
        "->n:i;p=&n",
        "p=&2",
        "text:\"local\";p=first(&a,&text)",
        "p=first(&a,&\"short\")",
    ] {
        Case::new(&format!(
            "d:@\"debug\";first<&int32>:(p<&int32>,text<&string>){{->p}};a:7;p:=&a;i:=0;'loop{{p=&a;d.print(*p);{update};i=i+1;|i<2|'loop.restart()}}"
        ))
        .runs(b"7\n7\n");
    }
}

#[test]
pub fn reinitializing_a_static_site_never_revives_an_expired_header_value() {
    for source in [
        "a:1;p:=&a;i:=0;'loop{local:i;|i>0|v:*p;p=&local;i=i+1;|i<2|'loop.restart()}",
        "a:1;p:=&a;i:=0;'loop{->n:i;|i>0|v:*p;p=&n;i=i+1;|i<2|'loop.restart()}",
        "a:1;p:=&a;i:=0;'loop{p=(&2).{|i>0|v:*p;->$};i=i+1;|i<2|'loop.restart()}",
        "first<&int32>:(p<&int32>,text<&string>){->p};a:1;p:=&a;i:=0;'loop{text:\"local\";|i>0|v:*p;p=first(&a,&text);i=i+1;|i<2|'loop.restart()}",
        "first<&int32>:(p<&int32>,text<&string>){->p};a:1;p:=&a;i:=0;'loop{p=first(&a,&\"short\").{|i>0|v:*p;->$};i=i+1;|i<2|'loop.restart()}",
    ] {
        rejects(source);
    }
}

#[test]
pub fn overwriting_a_header_keeps_old_copies_and_their_lifetimes_separate() {
    Case::new(
        r#"
d:@"debug"
a:7;p:=&a;old:p;i:=0
'loop{
    p=&a
    local:i
    p=&local
    i=i+1
    |i<2|'loop.restart()
}
d.print(*old)
p=&a
d.print(*p)
"#,
    )
    .runs(b"7\n7\n");
    for source in [
        "a:1;p:=&a;saved:=&a;i:=0;'loop{local:i;p=&a;|i>0|v:*saved;p=&local;saved=p;p=&a;i=i+1;|i<2|'loop.restart()}",
        "a:1;p:=&a;saved:=&a;i:=0;'loop{->n:i;p=&a;|i>0|v:*saved;p=&n;saved=p;p=&a;i=i+1;|i<2|'loop.restart()}",
        "a:1;p:=&a;i:=0;'loop{local:i;|i>0|{old:p;p=&a;v:*old};p=&local;i=i+1;|i<2|'loop.restart()}",
    ] {
        rejects(source);
    }
}

#[test]
pub fn nested_restarts_keep_ancestor_owners_live_until_their_own_restart() {
    Case::new(
        r#"
d:@"debug"
a:1;p:=&a;i:=0
'outer{
    p=&a
    local:i+7
    ->n:i+9
    p=&local
    j:=0
    'inner{
        d.print(*p)
        p=&n
        j=j+1
        |j<2|'inner.restart()
    }
    i=i+1
    |i<2|'outer.restart()
}
p=&a
d.print(*p)
"#,
    )
    .runs(b"7\n9\n8\n10\n1\n");
    rejects(
        "a:1;p:=&a;i:=0;'outer{local:i;|i>0|v:*p;j:=0;'inner{p=&local;j=j+1;|j<2|'inner.restart()};i=i+1;|i<2|'outer.restart()}",
    );
}

#[test]
pub fn inner_restarts_preserve_live_enclosing_statement_temporaries_and_bounds() {
    Case::new(
        r#"
d:@"debug"
(&7).{
    p:=$;i:=0
    'loop{d.print(*p);p=$;i=i+1;|i<2|'loop.restart()}
}
first<&int32>:(p<&int32>,text<&string>){->p}
a:9
first(&a,&"live").{
    p:=$;i:=0
    'loop{d.print(*p);p=$;i=i+1;|i<2|'loop.restart()}
}
"#,
    )
    .runs(b"7\n7\n9\n9\n");
}

#[test]
pub fn expired_nested_payloads_allow_tags_and_scalars_but_reject_active_reads() {
    Case::new(
        r#"
d:@"debug"
<H>:<{view<&int32><null>;count<int32>}>
expired<H>:(&7).{->view<&int32><null>:$;->count:2}
empty<H>:{->count:1}
p:=&expired;i:=0
'loop{
    d.print(p.count)
    |p.view<&int32>|d.print("tag")
    |p.view<null>|d.print("empty")
    p=&empty
    i=i+1
    |i<2|'loop.restart()
}
"#,
    )
    .runs(b"2\ntag\n1\nempty\n");
    Case::new(
        r#"
d:@"debug"
<Row>:<{view<&int32><null>;n<int32>}>
<H>:<{item<Row><null>}>
expired<H>:(&7).{->item<Row><null>:{->view<&int32><null>:$;->n:2}}
empty<H>:{}
blank<H>:{->item<Row><null>:{->n:1}}
p:=&expired;i:=0
'loop{
    p=&blank
    copy:*p
    |copy.item<Row>|{
        row:copy.item~<Row>
        |row.view<null>|d.print("blank")
    }
    p=&empty
    none:*p
    |none.item<null>|d.print("empty")
    i=i+1
    |i<2|'loop.restart()
}
"#,
    )
    .runs(b"blank\nempty\nblank\nempty\n");
    for read in ["copy:*p", "view:p.view;|view<&int32>|v:*(view~<&int32>)"] {
        rejects(&format!(
            "<H>:<{{view<&int32><null>;count<int32>}}>;expired<H>:(&7).{{->view<&int32><null>:$;->count:2}};empty<H>:{{->count:1}};p:=&expired;i:=0;'loop{{{read};p=&empty;i=i+1;|i<2|'loop.restart()}}"
        ));
    }
}

#[test]
pub fn nested_expired_public_bounds_follow_payload_activity_and_selected_reads() {
    Case::new(
        r#"
d:@"debug"
<H>:<{view<&int32><null>;count<int32>}>
make<H>:(p<&int32>,text<&string>){->view:p;->count:2}
none<H>:(text<&string>){->count:1}
a:7;expired:make(&a,&"short");empty:none(&"short")
p:=&expired;i:=0
'loop{
    d.print(p.count)
    |p.view<&int32>|d.print("tag")
    p=&empty
    copy:*p
    |copy.view<null>|d.print(copy.count)
    i=i+1
    |i<2|'loop.restart()
}
"#,
    )
    .runs(b"2\ntag\n1\n1\n1\n");
    rejects(
        "<H>:<{view<&int32><null>}>;make<H>:(p<&int32>,text<&string>){->view:p};a:7;expired:make(&a,&\"short\");empty<H>:{};p:=&expired;i:=0;'loop{view:p.view;|view<&int32>|v:*(view~<&int32>);p=&empty;i=i+1;|i<2|'loop.restart()}",
    );
}

#[test]
pub fn expired_header_replay_evaluates_materialization_and_calls_once() {
    Case::new(
        r#"
d:@"debug"
first<&int32>:(p<&int32>,text<&string>){d.print("call");->p}
a:7;p:=&{d.print("entry");->1};i:=0
'loop{
    p=&a
    d.print(*p)
    p=&{d.print("owner");->i}
    p=first(&a,&{d.print("bound");->"short"})
    i=i+1
    |i<2|'loop.restart()
}
"#,
    )
    .runs(b"entry\n7\nowner\nbound\ncall\n7\nowner\nbound\ncall\n");
}
