use super::Case;

pub(crate) fn rejects(source: &str, code: &str) {
    let case = Case::new(source);
    for profile in ["debug", "release"] {
        let output = case.command("build", &["--json", "--profile", profile]);
        assert_eq!(output.status.code(), Some(1), "{profile}: {source}");
        let error = String::from_utf8_lossy(&output.stderr);
        let primary = error
            .split("\"code\":\"")
            .nth(1)
            .and_then(|part| part.split('"').next());
        assert_eq!(primary, Some(code), "{profile}: {source}: {error}");
    }
}

#[test]
pub fn scalar_stores_preserve_widths_boolean_layout_and_last_use() {
    Case::new(
        r#"
d:@"debug"
x:=1;p:&!x;*p=*p+1;d.print(*p);x=3;d.print(x)
b:=true;q:&!b;*q=false;d.print(*q);b=true;d.print(b)
i<int8>:=126;r:&!i;*r=*r+1;d.print(*r)
f<float32>:=1.5;s:&!f;*s=*s+1.0;d.print(*s)
u<uint64>:=18446744073709551614;t:&!u;*t=*t+1;d.print(*t)
"#,
    )
    .runs(b"2\n3\nfalse\ntrue\n127\n2.5\n18446744073709551615\n");
}

#[test]
pub fn moves_and_reinitialization_preserve_authority() {
    Case::new(
        r#"
d:@"debug"
a:=1;b:=2;p:=&!a;q:p;d.print(*q);p=&!b;*p=3;d.print(*p)
x:=4;parent:&!x;child:&*parent;next:parent;d.print(*child);*next=5;d.print(*next)
y:=6;root:&!y;leaf:&!*root;copy:root;*leaf=7;d.print(*leaf);*copy=8;d.print(*copy)
"#,
    )
    .runs(b"1\n3\n4\n5\n7\n8\n");
}

#[test]
pub fn shared_children_and_implicit_conversion_suspend_only_writes() {
    Case::new(
        r#"
d:@"debug"
x:=1;p:&!x;s:&*p;t:s;u:&*p;d.print(*p);d.print(*t);d.print(*u);*p=2
v<&int32>:p;d.print(*v);*p=3;d.print(*p)
"#,
    )
    .runs(b"1\n1\n1\n2\n3\n");
}

#[test]
pub fn exclusive_children_release_parents_after_their_last_use() {
    Case::new(
        r#"
d:@"debug"
x:=1;p:&!x;q:&!*p;r:&!*q;s:&*r;d.print(*s);*r=2;d.print(*r);*q=3;d.print(*q);*p=4;d.print(*p)
"#,
    )
    .runs(b"1\n2\n3\n4\n");
}

#[test]
pub fn indirect_stores_capture_the_pointer_before_rhs_replacement() {
    Case::new(
        r#"
d:@"debug"
a:=1;b:=2;p:=&!a
*(p)={d.print(10);p=&!b;d.print(20);->3}
d.print(a);d.print(*p)
'out{*p={b=4;d.print(b);'out.leave()}}
d.print(b)
"#,
    )
    .runs(b"10\n20\n3\n2\n4\n4\n");
}

#[test]
pub fn guarded_moves_preserve_continuing_predecessors() {
    Case::new(
        r#"
d:@"debug"
f<null>:(flag<boolean>){
 a:=1;b:=2;p:=&!a
 |flag|{q:p;d.print(*q);p=&!b}
 d.print(*p)
 x:=3;r:&!x
 |flag|{s:r;d.print(*s)}
 |!flag|{d.print(*r)}
}
f(true);f(false)
x:=4;p:&!x;v:false&&{q:p;->*q>0};d.print(*p)
"#,
    )
    .runs(b"1\n2\n3\n1\n3\n4\n");
}

#[test]
pub fn guarded_children_suspend_the_selected_owner() {
    Case::new(
        r#"
d:@"debug"
f<null>:(flag<boolean>){
 a:=1;b:=2;p:=&!a
 |flag|p=&!b
 s:&*p
 |flag|a=3
 |!flag|b=4
 d.print(*s);*p=5;d.print(a);d.print(b)
}
f(true);f(false)
"#,
    )
    .runs(b"2\n3\n5\n1\n5\n4\n");
}

#[test]
pub fn named_leave_preserves_completed_moves_and_reinitialization() {
    Case::new(
        r#"
d:@"debug"
f<null>:(flag<boolean>){
 a:=1;b:=2;p:=&!a
 'out{
  |flag|{q:p;d.print(*q);p=&!b;'out.leave()}
  *p=3
 }
 d.print(*p)
}
f(true);f(false)
"#,
    )
    .runs(b"1\n2\n3\n");
}

#[test]
pub fn moved_handles_fail_through_every_consuming_form() {
    for source in [
        "x:=1;p:&!x;q:p;v:*p",
        "x:=1;p:&!x;p;v:*p",
        "x:=1;p:&!x;q:(p);v:*p",
        "x:=1;p:&!x;q:p~<&!int32>;v:*p",
        "x:=1;p:&!x;q:p;*p=2",
        "x:=1;p:&!x;q:p;s:&*p",
        "x:=1;p:&!x;q:p;s:&!*p",
        "x:=1;p:&!x;q:p;v:p~<&!int32>",
    ] {
        rejects(source, "E301");
    }
}

#[test]
pub fn uncertain_moves_report_storage_unavailability() {
    for source in [
        "f<null>:(flag<boolean>){a:=1;p:&!a;|flag|{q:p;v:*q};v:*p}",
        "f<null>:(flag<boolean>){a:=1;b:=2;p:=&!a;'out{|flag|{q:p;v:*q;'out.leave()};p=&!b};v:*p}",
        "f<null>:(flag<boolean>){a:=1;p:&!a;v:flag&&{q:p;->*q>0};w:*p}",
        "f<null>:(flag<boolean>){a:=1;p:&!a;v:flag||{q:p;->*q>0};w:*p}",
    ] {
        rejects(source, "E309");
    }
}

#[test]
pub fn live_exclusive_loans_exclude_external_access_and_acquisition() {
    for source in [
        "x:=1;p:&!x;v:x;w:*p",
        "x:=1;p:&!x;x=2;v:*p",
        "x:=1;p:&!x;q:&!x;v:*p;w:*q",
        "x:=1;s:&x;p:&!x;v:*s",
        "x:=1;p:&!x;s:&x;v:*p",
        "x:=1;p:&!x;q:&!*p;v:x;w:*q",
        "x:=1;p:&!x;s:&*p;v:x;w:*s",
        "x:=1;p:&!x;*p={x=2;->3}",
        "first<&int32>:(a<&int32>,b<&int32>){->a};a:=1;b:=2;s:first(&a,&b);p:&!b;v:*s",
    ] {
        rejects(source, "E302");
    }
}

#[test]
pub fn live_children_suspend_incompatible_parent_and_sibling_access() {
    for source in [
        "x:=1;p:&!x;s:&*p;*p=2;v:*s",
        "x:=1;p:&!x;s:&*p;t:s;*p=2;v:*t",
        "x:=1;p:&!x;s<&int32>:p;*p=2;v:*s",
        "x:=1;p:&!x;q:&!*p;v:*p;w:*q",
        "x:=1;p:&!x;q:&!*p;*p=2;w:*q",
        "x:=1;p:&!x;q:&!*p;s:&*p;w:*q",
        "x:=1;p:&!x;s:&*p;q:&!*p;w:*s",
        "x:=1;p:&!x;q:&!*p;r:&!*p;w:*q",
        "x:=1;p:&!x;q:&!*p;next:p;*next=2;v:*q",
    ] {
        rejects(source, "E302");
    }
}

#[test]
pub fn exclusive_lifetimes_and_mutability_keep_exact_diagnostics() {
    rejects("a:=1;p:=&!a;{b:=2;p=&!b};v:*p", "E303");
    for source in [
        "x:1;p:&!x",
        "x:=1;p:&x;*p=2",
        "x:=1;p:&x;q:&!*p",
        "x:=1;y:=2;p:&!x;p=&!y",
    ] {
        rejects(source, "E305");
    }
}

#[test]
pub fn unsupported_shapes_and_shared_ancestry_crossings_stay_gated() {
    for source in [
        "f<{p<&!int32>}>:(p<&!int32>){->p:p}",
        "x:=1;p:&!x;h:{->p:p}",
        "x:=1;p:&!x;h:&p",
        "x:=1;p:&!x;s:&*p;h:&s",
        "x:=1;p:&!x;s:&*p;h:{->s:s}",
        "x:=1;p:&!x;s:&*p;t:&({->s})",
        "x:=1;p:&!x;t:p.{->*$}",
        "x:=1;p:&!x;t:(&*p).{->*$}",
        "x:=1;p:&!x;'again{'again.restart()}",
        "x:={->n:=1};p:&!x",
        "f<&int32>:(s<&int32>,extra<string>){->s};x:=1;p:&!x;s:&*p;v:f(s,\"\")",
        "f<&int32>:(s<&int32>,extra<string>){->s};x:=1;p:&!x;v:f(p,\"\")",
        "x:=1;p:&!x;q:&!*p;v:p==q",
    ] {
        rejects(source, "B001");
    }
}

#[test]
pub fn panic_skips_the_final_store_without_retaining_a_future_access() {
    let case = Case::new(r#"d:@"debug";x:=1;p:&!x;*p={x=2;d.print(x);d.panic("stop")}"#);
    for profile in ["debug", "release"] {
        let output = case.command("run", &["--profile", profile]);
        assert_eq!(output.stdout, b"2\n");
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).starts_with("panic[P006]: stop"));
    }
}

#[test]
pub fn indirect_targets_keep_their_captured_child_authority() {
    Case::new(r#"d:@"debug";x:=1;p:&!x;*(&!*p)={d.print(10);->2};d.print(*p)"#).runs(b"10\n2\n");
    Case::new(r#"d:@"debug";x:=1;p:&!x;*p={q:p;d.print(*q);->2};d.print(x)"#).runs(b"1\n2\n");
    rejects("x:=1;p:&!x;*(&!*p)={v:*p;->2}", "E302");
    rejects("x:=1;p:&!x;*p={q:p;->2};v:*p", "E301");
}

#[test]
pub fn bounded_parent_walks_execute_or_reject_without_partial_proofs() {
    for (count, code) in [(16, None), (512, Some("B001"))] {
        let mut source = String::from("d:@\"debug\";x:=1;p0:&!x;");
        for index in 1..count {
            source.push_str(&format!("p{index}:&!*p{};", index - 1));
        }
        source.push_str(&format!("*p{}=2;d.print(*p{});", count - 1, count - 1));
        if let Some(code) = code {
            rejects(&source, code);
        } else {
            Case::new(&source).runs(b"2\n");
        }
    }
}
