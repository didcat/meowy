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
pub fn guarded_reference_reads_keep_both_paths_and_prior_copies() {
    Case::new(
        r#"
d:@"debug"
choose<int32>:(flag<boolean>){
    a:1;b:2;p:=&a;old:p
    |flag|p=&b
    d.print(*old)
    ->*p
}
d.print(choose(false));d.print(choose(true))
nested<int32>:(first<boolean>,second<boolean>){
    a:3;b:4;c:5;p:=&a;q:=&a
    |first|{p=&b;|second|q=&c}
    d.print(*q)
    ->*p
}
d.print(nested(false,false));d.print(nested(false,true))
d.print(nested(true,false));d.print(nested(true,true))
"#,
    )
    .runs(b"1\n1\n1\n2\n3\n3\n3\n3\n3\n4\n5\n4\n");
}

#[test]
pub fn guarded_owner_writes_follow_only_live_selected_versions() {
    Case::new(
        r#"
d:@"debug"
choose<int32>:(flag<boolean>){
    a:=1;b:=2;p:=&a
    |flag|p=&b
    |flag|a=3
    |!flag|b=4
    ->*p
}
d.print(choose(false));d.print(choose(true))
unused<null>:(flag<boolean>){a:=1;b:2;p:=&a;q:=&b;q=&b;|flag|a=2}
unused(false);unused(true)
"#,
    )
    .runs(b"1\n2\n");
    for source in [
        "f<int32>:(flag<boolean>){a:=1;b:2;p:=&a;|flag|p=&b;a=3;->*p}",
        "f<int32>:(flag<boolean>){a:1;b:=2;p:=&a;|flag|p=&b;|flag|b=3;->*p}",
        "f<int32>:(flag<boolean>){a:=1;b:2;p:=&a;old:p;|flag|p=&b;|flag|a=3;->*old}",
        "f<int32>:(start<boolean>){flag:=start;a:=1;b:2;p:=&a;|flag|p=&b;flag=false;|!flag|a=3;->*p}",
    ] {
        rejects(source, "E302");
    }
}

#[test]
pub fn guarded_cell_borrows_keep_physical_storage_and_final_reads() {
    Case::new(
        r#"
d:@"debug"
choose<int32>:(flag<boolean>){
    a:1;b:2;p:=&a;cell:&p
    |flag|p={d.print(**cell);->&b}
    |!flag|d.print(**cell)
    ->*p
}
d.print(choose(false));d.print(choose(true))
copy<int32>:(flag<boolean>){a:3;b:4;p:=&a;old:*(&p);|flag|p=&b;->*old}
d.print(copy(false));d.print(copy(true))
"#,
    )
    .runs(b"1\n1\n1\n2\n3\n3\n");
    for source in [
        "f<int32>:(flag<boolean>){a:1;b:2;p:=&a;cell:&p;|flag|p=&b;->**cell}",
        "<R>:<&int32>;load<R>:(cell<&R>){->*cell};f<int32>:(flag<boolean>){a:1;b:2;p:=&a;old:load(&p);|flag|p=&b;->*old}",
    ] {
        rejects(source, "E302");
    }
}

#[test]
pub fn guarded_reference_lifetimes_validate_only_the_demanded_branch() {
    Case::new(
        r#"
d:@"debug"
choose<int32>:(flag<boolean>){
    a:7;p:=&a
    |flag|p=&1
    |!flag|d.print(*p)
    p=&a
    ->*p
}
d.print(choose(false));d.print(choose(true))
first<&int32>:(p<&int32>,q<&string>){->p}
bounds<int32>:(flag<boolean>){
    a:8;p:=&a
    |flag|p=first(&a,&"temporary")
    |!flag|d.print(*p)
    p=&a
    ->*p
}
d.print(bounds(false));d.print(bounds(true))
"#,
    )
    .runs(b"7\n7\n7\n8\n8\n8\n");
    for source in [
        "f<int32>:(flag<boolean>){a:1;p:=&a;|flag|p=&2;->*p}",
        "f<int32>:(flag<boolean>){a:1;p:=&a;|flag|{b:2;p=&b};->*p}",
        "first<&int32>:(p<&int32>,q<&string>){->p};f<int32>:(flag<boolean>){a:1;p:=&a;|flag|p=first(&a,&\"temporary\");->*p}",
        "f<&int32>:(input<&int32>,flag<boolean>){a:1;p:=input;|flag|p=&a;->p}",
    ] {
        rejects(source, "E303");
    }
}

#[test]
pub fn guarded_assignments_keep_effect_order_and_skip_panicking_continuations() {
    Case::new(
        r#"
d:@"debug"
same<boolean>:(a<&int32>,b<&int32>){->a==b}
choose<null>:(flag<boolean>){
    a:1;b:2;p:=&a
    |{d.print("condition");p=&b;->flag}|p={d.print("rhs");->&a}
    d.print(*p)
    d.print(same(p,{|flag|p=&b;->p}))
    d.print(*p)
}

choose(false);choose(true)
"#,
    )
    .runs(b"condition\n2\ntrue\n2\ncondition\nrhs\n1\nfalse\n2\n");
    rejects(
        "d:@\"debug\";f<int32>:(flag<boolean>){a:=1;b:2;p:=&a;|flag|{p=&b;d.panic(\"stop\")};a=3;->*p}",
        "E302",
    );
    let source = "d:@\"debug\";f<int32>:(flag<boolean>){a:1;b:2;p:=&a;|flag|p={d.print(\"rhs\");d.panic(\"stop\");->&b};->*p};d.print(f(false));d.print(f(true))";
    let call = "d.panic(\"stop\")";
    let start = source.find(call).unwrap();
    let end = start + call.len();
    for profile in ["debug", "release"] {
        let output = Case::new(source).command("run", &["--profile", profile]);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, b"1\nrhs\n");
        assert_eq!(
            output.stderr,
            format!("panic[P006]: stop at bytes {start}..{end}\n").as_bytes()
        );
    }
}

#[test]
pub fn short_circuit_reference_writes_merge_executed_and_skipped_values() {
    Case::new(
        r#"
d:@"debug"
both<int32>:(flag<boolean>){
    a:1;b:2;p:=&a
    value:flag&&{d.print("and");p=&b;->true}
    d.print(value)
    ->*p
}
either<int32>:(flag<boolean>){
    a:3;b:4;p:=&a
    value:flag||{d.print("or");p=&b;->true}
    d.print(value)
    ->*p
}
d.print(both(false));d.print(both(true))
d.print(either(false));d.print(either(true))
nested<int32>:(first<boolean>,second<boolean>){
    a:5;b:6;p:=&a
    value:first&&(!second||{p=&b;->true})
    ->*p
}
d.print(nested(false,false));d.print(nested(false,true))
d.print(nested(true,false));d.print(nested(true,true))
"#,
    )
    .runs(b"false\n1\nand\ntrue\n2\nor\ntrue\n4\ntrue\n3\n5\n5\n5\n6\n");
    for source in [
        "f<int32>:(flag<boolean>){a:=1;b:2;p:=&a;value:flag&&{p=&b;->true};a=3;->*p}",
        "f<int32>:(flag<boolean>){a:=1;b:2;p:=&a;value:flag||{p=&b;->true};a=3;->*p}",
        "f<int32>:(flag<boolean>){a:1;b:2;p:=&a;cell:&p;value:flag&&{p=&b;->true};->**cell}",
    ] {
        rejects(source, "E302");
    }
}

#[test]
pub fn complementary_overwrites_remove_expired_versions_without_reading_them() {
    Case::new(
        r#"
d:@"debug"
choose<int32>:(flag<boolean>){
    a:7;b:8;p:=&1
    |flag|p=&a
    |!flag|p=&b
    ->*p
}
d.print(choose(false));d.print(choose(true))
guarded<int32>:(flag<boolean>){
    a:9;p:=&a
    skipped:flag||{p=&2;->true}
    |flag|d.print(*p)
    p=&a
    ->*p
}
d.print(guarded(false));d.print(guarded(true))
"#,
    )
    .runs(b"8\n7\n9\n9\n9\n");
    rejects(
        "f<int32>:(flag<boolean>){a:1;p:=&2;|flag|p=&a;->*p}",
        "E303",
    );
}

#[test]
pub fn guarded_versions_preserve_reference_carriers_and_public_bounds() {
    Case::new(
        r#"
d:@"debug"
<H>:<{view<&int32><null>;count<int32>}>
choose<null>:(flag<boolean>){
    a:7
    empty<H>:{->count:3}
    full<H>:{->view:&a;->count:4}
    p:=&empty
    |flag|p=&full
    copy:*p
    |copy.view<&int32>|d.print(*(copy.view~<&int32>))
    d.print(copy.count)
}
choose(false);choose(true)
pick<&int32>:(a<&int32>,b<&int32>,flag<boolean>){p:=a;|flag|p=b;->p}
a:8;b:9
d.print(*(pick(&a,&b,false)));d.print(*(pick(&a,&b,true)))
"#,
    )
    .runs(b"3\n7\n4\n8\n9\n");
    rejects(
        "pick<&int32>:(a<&int32>,b<&int32>,flag<boolean>){p:=a;|flag|p=b;->p};a:1;b:=2;value:pick(&a,&b,false);b=3;read:*value",
        "E302",
    );
}
