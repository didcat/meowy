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
pub fn restart_headers_preserve_initial_backedge_and_copied_reference_values() {
    Case::new(
        r#"
d:@"debug"
a:7;b:9;p:=&a;old:p;count:=0
'loop{
    d.print(*p)
    p=&b
    count=count+1
    |count<3|'loop.restart()
}
d.print(*old);d.print(*p)
"#,
    )
    .runs(b"7\n9\n9\n7\n9\n");
}

#[test]
pub fn restart_loans_protect_initial_and_future_iteration_owners() {
    for source in [
        "a:=1;b:2;p:=&a;i:=0;'loop{a=3;v:*p;p=&b;i=i+1;|i<2|'loop.restart()}",
        "a:1;b:=2;p:=&a;i:=0;'loop{v:*p;p=&b;b=3;i=i+1;|i<2|'loop.restart()}",
        "a:1;b:=2;p:=&a;later:=false;'loop{|later|{v:*p;'loop.leave()};p=&b;|!later|b=3;later=true;'loop.restart()}",
        "a:=1;b:2;p:=&a;old:p;i:=0;'loop{p=&b;i=i+1;|i<2|'loop.restart()};a=3;v:*old",
    ] {
        rejects(source, "E302");
    }
}

#[test]
pub fn iteration_local_reference_bindings_are_reinitialized_before_each_use() {
    Case::new(
        r#"
d:@"debug"
count:=0
'loop{
    a:count;b:count+10;p:=&a;copy:p
    p=&b
    d.print(*copy);d.print(*p)
    count=count+1
    |count<2|'loop.restart()
}
"#,
    )
    .runs(b"0\n10\n1\n11\n");
}

#[test]
pub fn restart_from_rhs_retains_completed_updates_and_skips_the_outer_store() {
    Case::new(
        r#"
d:@"debug"
a:1;b:2;c:3;p:=&a;count:=0
'loop{
    d.print(*p)
    count=count+1
    p={d.print("rhs");p=&b;|count<2|'loop.restart();->&c}
    d.print(*p)
}
"#,
    )
    .runs(b"1\nrhs\n2\nrhs\n3\n");
}

#[test]
pub fn restart_headers_preserve_preloop_precision_without_synthetic_reads() {
    Case::new(
        r#"
d:@"debug"
a:1;b:=2;p:=&a;b=3;count:=0
'loop{d.print(*p);p=&b;count=count+1;|count<2|'loop.restart()}
after:=1;other:=2;unused:=&after;count=0
'again{after=3;unused=&other;other=4;count=count+1;|count<2|'again.restart()}
d.print(after);d.print(other)
"#,
    )
    .runs(b"1\n3\n3\n4\n");
}

#[test]
pub fn restart_cell_loans_protect_stores_without_extending_finished_reads() {
    Case::new(
        r#"
d:@"debug"
a:1;b:2;p:=&a;count:=0
'loop{
    cell:&p
    d.print(**cell)
    p=&b
    count=count+1
    |count<2|'loop.restart()
}
"#,
    )
    .runs(b"1\n2\n");
    for source in [
        "a:1;b:2;p:=&a;cell:&p;i:=0;'loop{p=&b;i=i+1;|i<2|'loop.restart()};v:**cell",
        "<R>:<&int32>;load<R>:(p<&R>){->*p};a:1;b:2;p:=&a;old:load(&p);i:=0;'loop{p=&b;i=i+1;|i<2|'loop.restart()};v:*old",
    ] {
        rejects(source, "E302");
    }
}

#[test]
pub fn rotating_and_nested_restart_headers_converge_to_all_reaching_sources() {
    Case::new(
        r#"
d:@"debug"
a:1;b:2;c:3;p:=&a;q:=&b;r:=&c;count:=0
'loop{
    d.print(*p);d.print(*q);d.print(*r)
    p=q;q=r;r=&a
    count=count+1
    |count<3|'loop.restart()
}
p=&a;count=0
'outer{
    inner:=0
    'inner{
        d.print(*p)
        p=&b
        inner=inner+1
        |inner<2|'inner.restart()
    }
    p=&c
    count=count+1
    |count<2|'outer.restart()
}
d.print(*p)
"#,
    )
    .runs(b"1\n2\n3\n2\n3\n1\n3\n1\n1\n1\n2\n3\n2\n3\n");
}

#[test]
pub fn restart_carried_expiry_allows_unread_values_without_reviving_storage() {
    rejects(
        "d:@\"debug\";a:1;p:=&a;i:=0;'loop{local:i;|i>0|d.print(*p);p=&local;i=i+1;|i<2|'loop.restart()}",
        "E303",
    );
    for source in [
        "a:1;p:=&a;i:=0;'loop{->n:2;p=&n;i=i+1;|i<2|'loop.restart()}",
        "a:1;p:=&a;i:=0;'loop{p=&2;i=i+1;|i<2|'loop.restart()}",
        "a:1;p:=&2;i:=0;'loop{p=&a;i=i+1;|i<2|'loop.restart()}",
        "a:1;holder:{->view:&a};p:=&holder;i:=0;'loop{local:{->view:&a};p=&local;i=i+1;|i<2|'loop.restart()}",
        "first<&int32>:(p<&int32>,q<&string>){->p};a:1;p:=&a;i:=0;'loop{text:\"local\";p=first(&a,&text);i=i+1;|i<2|'loop.restart()}",
        "first<&int32>:(p<&int32>,q<&string>){->p};a:1;p:=first(&a,&\"short\");i:=0;'loop{p=&a;i=i+1;|i<2|'loop.restart()}",
    ] {
        Case::new(source).runs(b"");
    }
}

#[test]
pub fn restart_headers_support_reference_free_records_lists_and_unions() {
    Case::new(
        r#"
d:@"debug"
a:{->value:1};b:{->value:2};p:=&a;count:=0
'record{d.print(p.value);p=&b;count=count+1;|count<2|'record.restart()}
left<int32[2]>:[3];right<int32[2]>:[4,5];items:=&left;count=0
'list{d.print((*items)[1]);items=&right;count=count+1;|count<2|'list.restart()}
first<int32><null>:6;last<int32><null>:null;value:=&first;count=0
'union{
    copy:*value
    |copy<int32>|d.print(copy~<int32>)
    |copy<null>|d.print("empty")
    value=&last
    count=count+1
    |count<2|'union.restart()
}
"#,
    )
    .runs(b"1\n2\n3\n4\n6\nempty\n");
}

#[test]
pub fn restart_calls_keep_public_bounds_and_skip_unentered_calls() {
    Case::new(
        r#"
d:@"debug"
first<&int32>:(p<&int32>,text<&string>){->p}
a:1;b:2;text:="old";p:=first(&a,&text);count:=0
'loop{d.print(*p);p=&b;count=count+1;|count<2|'loop.restart()}
text="new"
d.print(text)
same<boolean>:(a<&int32>,b<&int32>){d.print("call");->a==b}
p=&a;count=0
'call{
    value:same(p,{p=&b;count=count+1;|count<2|'call.restart();->p})
    d.print(value)
}
"#,
    )
    .runs(b"1\n2\nnew\ncall\ntrue\n");
    rejects(
        "first<&int32>:(p<&int32>,text<&string>){->p};a:1;b:2;text:=\"old\";p:=first(&a,&text);i:=0;'loop{text=\"new\";value:*p;p=&b;i=i+1;|i<2|'loop.restart()}",
        "E302",
    );
}

#[test]
pub fn an_inner_restart_preserves_an_ancestor_result_slot_owner() {
    Case::new(
        r#"
d:@"debug"
a:1;p:=&a
'owner{
    ->n:7
    p=&n
    count:=0
    'inner{d.print(*p);p=&n;count=count+1;|count<2|'inner.restart()}
    p=&a
}
d.print(*p)
"#,
    )
    .runs(b"7\n7\n1\n");
    Case::new("a:1;p:=&a;i:=0;'owner{->n:7;p=&n;i=i+1;|i<2|'owner.restart()}").runs(b"");
}
