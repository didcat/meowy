use super::Case;
use super::exclusive_references::rejects;

#[test]
pub fn anonymous_exclusive_results_move_and_reinitialize_holders() {
    Case::new(
        r#"
d:@"debug"
x:=1;p:=&!x;r:{->p};*r=2;d.print(*r);p=&!x;*p=3;d.print(*p)
y:=4;q:&!y;s:{t:q;->{->t}};d.print(*s)
"#,
    )
    .runs(b"2\n3\n4\n");
    for source in [
        "x:=1;p:&!x;r:{->p};v:*p",
        "x:=1;p:&!x;r:{->p;v:*p}",
        "x:=1;p:&!x;{->p};v:*p",
        "x:=1;p:&!x;r:{->{->(p)<&!int32>}};v:*p",
    ] {
        rejects(source, "E301");
    }
}

#[test]
pub fn shared_results_and_expected_conversions_keep_parent_authority() {
    Case::new(
        r#"
d:@"debug"
x:=1;p:&!x;r:{->&*p};s:r;d.print(*p);d.print(*s);*p=2
v<&int32>:{->p};d.print(*v);*p=3;d.print(*p)
"#,
    )
    .runs(b"1\n1\n2\n3\n");
    rejects("x:=1;p:&!x;r:{->&*p};*p=2;v:*r", "E302");
    rejects("x:=1;p:&!x;r<&int32>:{->p};*p=2;v:*r", "E302");
}

#[test]
pub fn retained_emissions_protect_the_result_through_block_completion() {
    for source in [
        "x:=1;p:&!x;r:{->&*p;*p=2}",
        "x:=1;p:&!x;r:{->&!*p;v:*p}",
        "x:=1;r:{->&!x;x=2}",
        "x:=1;p:&!x;r:{->&*p;x=2};v:*r",
    ] {
        rejects(source, "E302");
    }
    Case::new(r#"d:@"debug";x:=1;p:&!x;r:{s:&*p;->s;d.print(*p)};d.print(*r);*p=2;d.print(*p)"#)
        .runs(b"1\n1\n2\n");
}

#[test]
pub fn guarded_results_transfer_only_the_selected_holder() {
    Case::new(
        r#"
d:@"debug"
f<null>:(flag<boolean>){
 a:=1;b:=2;p:&!a;q:&!b
 r:{|flag|->p;|!flag|->q}
 |flag|{*q=3;d.print(*q)}
 |!flag|{*p=4;d.print(*p)}
 *r=5;d.print(*r)
}
f(true);f(false)
"#,
    )
    .runs(b"3\n5\n4\n5\n");
    rejects(
        "f<null>:(flag<boolean>){a:=1;b:=2;p:&!a;q:&!b;r:{|flag|->p;|!flag|->q};v:*p}",
        "E309",
    );
}

#[test]
pub fn named_leave_returns_the_initialized_result_once() {
    Case::new(
        r#"
d:@"debug"
f<null>:(flag<boolean>){
 a:=1;b:=2;p:&!a;q:&!b
 r:'pick{
  |flag|{'pick->p;d.print(10);'pick.leave()}
  ->q;d.print(20)
 }
 d.print(*r)
}
f(true);f(false)
"#,
    )
    .runs(b"10\n1\n20\n2\n");
}

#[test]
pub fn cancelled_results_preserve_moves_without_retaining_future_loans() {
    Case::new(r#"d:@"debug";x:=1;p:&!x;'out{r:{->p;x=2;d.print(x);'out.leave()}};x=3;d.print(x);a:=4;q:&!a;'out{r:{->&*q;*q=5;d.print(*q);'out.leave()}};*q=6;d.print(*q)"#).runs(b"2\n3\n5\n6\n");
    rejects("x:=1;p:&!x;'out{r:{->p;'out.leave()}};v:*p", "E301");
}

#[test]
pub fn early_leave_keeps_completed_rhs_replacements() {
    Case::new(
        r#"
d:@"debug"
a:=1;b:=2;p:=&!a
'out{p={old:p;d.print(*old);p=&!b;'out.leave()}}
d.print(*p)
"#,
    )
    .runs(b"1\n2\n");
    rejects("a:=1;p:=&!a;'out{p={old:p;'out.leave()}};v:*p", "E301");
}

#[test]
pub fn block_result_targets_are_captured_once_before_rhs_effects() {
    Case::new(
        r#"
d:@"debug"
a:=1;b:=2;p:=&!a
*{d.print(10);->&!*p}={d.print(20);p=&!b;->3}
d.print(a);d.print(*p)
'out{*{->&!*p}={b=4;'out.leave()}}
d.print(b)
"#,
    )
    .runs(b"10\n20\n3\n2\n4\n");
    rejects("x:=1;p:&!x;*{->&!*p}={*p=2;->3}", "E302");
    rejects("x:=1;p:&!x;*{->p}={v:*p;->2}", "E301");
}

#[test]
pub fn call_and_block_results_preserve_parent_chains_and_public_bounds() {
    Case::new(
        r#"
d:@"debug"
id<&!int32>:(p<&!int32>){->{->p}}
share<&int32>:(p<&int32>){->p}
x:=1;p:&!x;r:{->id({->&!*p})};*r=2;d.print(*r)
s:{->share({->&*p})};d.print(*s);*p=3;d.print(*p)
"#,
    )
    .runs(b"2\n2\n3\n");
    rejects(
        "first<&!int32>:(p<&!int32>,b<&boolean>){->p};x:=1;b:=true;r:{->first(&!x,&b)};b=false;v:*r",
        "E302",
    );
    rejects(
        "first<&!int32>:(p<&!int32>,b<&boolean>){->p};x:=1;r:{b:true;->first(&!x,&b)}",
        "E303",
    );
}

#[test]
pub fn nested_owners_and_temporaries_cannot_escape_completed_blocks() {
    for source in [
        "r:{x:=1;->&!x}",
        "r:{x:=1;p:&!x;->&*p}",
        "r:{->{x:=1;->&!x}}",
        "r:'out{{x:=1;'out->&!x;'out.leave()}}",
    ] {
        rejects(source, "E303");
    }
}

#[test]
pub fn scalar_widths_and_boolean_layout_survive_block_results() {
    Case::new(r#"d:@"debug";b:=true;*{->&!b}=false;d.print(b);i<int8>:=126;p:{->&!i};*p=*p+1;d.print(*p);f<float32>:=1.5;*{->&!f}=2.5;d.print(f)"#).runs(b"false\n127\n2.5\n");
}

#[test]
pub fn emission_initialization_and_mutability_rules_remain_explicit() {
    rejects("x:=1;p:&!x;r:{->p;->&!x}", "E205");
    rejects(
        "f<null>:(flag<boolean>){x:=1;p:&!x;r<&!int32>:{|flag|->p}}",
        "E204",
    );
    rejects("x:1;r:{->&!x}", "E305");
    rejects("x:=1;r:{->&x};*r=2", "E305");
}

#[test]
pub fn named_carriers_cells_dispatch_and_restart_results_stay_gated() {
    for source in [
        "x:=1;p:&!x;r:{->view:p}",
        "x:=1;p:&!x;'out{r:{->view:p;'out.leave()}}",
        "x:=1;p:&!x;r:{->view:&*p}",
        "x:=1;p:&!x;r:{->&*p};s:&r",
        "x:=1;p:&!x;r:1.{->p}",
        "x:=1;p:&!x;r:1.{->&*p}",
        "x:=1;p:&!x;r:p.{->$}",
        "x:=1;p:&!x;r:'again{->p;'again.restart()}",
    ] {
        rejects(source, "B001");
    }
}

#[test]
pub fn panic_discards_emitted_authority_without_undoing_prior_effects() {
    let case = Case::new(r#"d:@"debug";x:=1;p:&!x;r:{->p;x=2;d.print(x);d.panic("stop")}"#);
    for profile in ["debug", "release"] {
        let output = case.command("run", &["--profile", profile]);
        assert!(!output.status.success());
        assert_eq!(output.stdout, b"2\n");
        assert!(String::from_utf8_lossy(&output.stderr).starts_with("panic[P006]: stop"));
    }
}

#[test]
pub fn short_circuit_block_moves_preserve_guarded_availability() {
    Case::new(
        r#"
d:@"debug"
f<null>:(flag<boolean>){
 x:=1;p:&!x;v:flag&&(*{->p}>0);d.print(v)
 |!flag|d.print(*p)
}
f(true);f(false)
y:=2;q:&!y;v:false&&(*{->q}>0);d.print(*q)
"#,
    )
    .runs(b"true\nfalse\n1\n2\n");
    rejects(
        "f<null>:(flag<boolean>){x:=1;p:&!x;v:flag&&(*{->p}>0);w:*p}",
        "E309",
    );
}
