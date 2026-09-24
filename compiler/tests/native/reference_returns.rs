use super::Case;
use super::exclusive_references::rejects;

#[test]
pub fn exclusive_identity_returns_transfer_scalar_authority() {
    Case::new(
        r#"
d:@"debug"
id<&!int32>:(p<&!int32>){->p}
x:=1;p:&!x;r:id(p);*r=2;d.print(*r);x=3;d.print(x)
a:=4;q:&!a;s:id(&!*q);*s=5;d.print(*s);*q=6;d.print(*q)
"#,
    )
    .runs(b"2\n3\n5\n6\n");
}

#[test]
pub fn shared_returns_preserve_child_identity_and_parent_reads() {
    Case::new(
        r#"
d:@"debug"
id<&int32>:(p<&int32>){->p}
share<&int32>:(p<&!int32>){->&*p}
x:=1;p:&!x;s:id(p);t:s;d.print(*p);d.print(*t);*p=2
u:share(&!*p);d.print(*u);*p=3;d.print(*p)
"#,
    )
    .runs(b"1\n1\n2\n3\n");
}

#[test]
pub fn nested_and_recursive_returns_keep_captured_parents() {
    Case::new(
        r#"
d:@"debug"
id<&!int32>:(p<&!int32>){->p}
wrap<&!int32>:(p<&!int32>){->id(p)}
walk<&!int32>:(p<&!int32>,n<int32>) 'out {
 |n>0|{'out->walk(p,n-1);'out.leave()}
 ->p
}
x:=1;p:&!x;r:walk(wrap(&!*p),3);*r=2;d.print(*r);*p=3;d.print(*p)
"#,
    )
    .runs(b"2\n3\n");
}

#[test]
pub fn guarded_input_selection_executes_both_returns() {
    Case::new(
        r#"
d:@"debug"
pick<&!int32>:(p<&!int32>,q<&!int32>,flag<boolean>){|flag|->p;|!flag|->q}
f<null>:(flag<boolean>){
 a:=1;b:=2;p:&!a;q:&!b
 r:pick(&!*p,&!*q,flag);*r=9;d.print(*r)
 *p=*p+1;*q=*q+1;d.print(*p);d.print(*q)
}
f(true);f(false)
"#,
    )
    .runs(b"9\n10\n3\n9\n2\n10\n");
}

#[test]
pub fn scalar_result_widths_and_boolean_storage_remain_correct() {
    Case::new(
        r#"
d:@"debug"
small<&!int8>:(p<&!int8>){->&!*p}
flag<&!boolean>:(p<&!boolean>){->p}
float<&!float32>:(p<&!float32>){->p}
i<int8>:=126;*(small(&!i))=127;d.print(i)
b:=true;*(flag(&!b))=false;d.print(b)
f<float32>:=1.5;*(float(&!f))=2.5;d.print(f)
"#,
    )
    .runs(b"127\nfalse\n2.5\n");
}

#[test]
pub fn lifetime_only_inputs_allow_reads_but_retain_write_protection() {
    Case::new(
        r#"
d:@"debug"
first<&!int32>:(p<&!int32>,q<&boolean>){->p}
x:=1;b:=true;r:first(&!x,&b);d.print(b);*r=2;d.print(*r);b=false;d.print(b)
"#,
    )
    .runs(b"true\n2\nfalse\n");
    rejects(
        "first<&!int32>:(p<&!int32>,q<&boolean>){->p};x:=1;b:=true;r:first(&!x,&b);b=false;v:*r",
        "E302",
    );
    rejects(
        "first<&!int32>:(p<&!int32>,q<&boolean>){->p};x:=1;r:=&!x;{b:=true;r=first(r,&b)};v:*r",
        "E303",
    );
    rejects(
        "first<&!int32>:(p<&!int32>,q<&boolean>){->p};wrap<&!int32>:(p<&!int32>){b:true;->first(p,&b)}",
        "E303",
    );
}

#[test]
pub fn returned_children_suspend_parent_and_sibling_access() {
    for source in [
        "id<&!int32>:(p<&!int32>){->p};x:=1;p:&!x;r:id(&!*p);v:*p;w:*r",
        "id<&!int32>:(p<&!int32>){->p};x:=1;p:&!x;r:id(&!*p);*p=2;w:*r",
        "id<&int32>:(p<&int32>){->p};x:=1;p:&!x;r:id(p);*p=2;w:*r",
        "id<&int32>:(p<&int32>){->p};x:=1;p:&!x;r:id(p);s:r;*p=2;w:*s",
        "id<&!int32>:(p<&!int32>){->p};x:=1;p:&!x;r:id(&!*p);s:&!*p;w:*r",
        "pick<&!int32>:(p<&!int32>,q<&!int32>,flag<boolean>){|flag|->p;|!flag|->q};a:=1;b:=2;p:&!a;q:&!b;r:pick(&!*p,&!*q,true);*q=3;v:*r",
    ] {
        rejects(source, "E302");
    }
}

#[test]
pub fn moving_returned_handles_and_inputs_reports_storage_errors() {
    for source in [
        "id<&!int32>:(p<&!int32>){->p};x:=1;p:&!x;r:id(p);v:*p",
        "id<&!int32>:(p<&!int32>){->p};x:=1;r:id(&!x);s:r;v:*r",
        "bad<&!int32>:(p<&!int32>){->p;v:*p}",
    ] {
        rejects(source, "E301");
    }
    rejects(
        "id<&!int32>:(p<&!int32>){->p};f<null>:(flag<boolean>){x:=1;p:&!x;|flag|{r:id(p)};v:*p}",
        "E309",
    );
}

#[test]
pub fn retained_emissions_protect_returned_loans_until_function_exit() {
    rejects("bad<&int32>:(p<&!int32>){->&*p;*p=2}", "E302");
    rejects("bad<&!int32>:(p<&!int32>){->&!*p;v:*p}", "E302");
    rejects("bad<&!int32>:(){x:=1;->&!x}", "E303");
    rejects("bad<&int32>:(){x:=1;p:&!x;->&*p}", "E303");
}

#[test]
pub fn call_entry_still_checks_aliases_and_suspended_parents() {
    rejects(
        "id<&!int32>:(p<&!int32>){->p};x:=1;p:&!x;s:&*p;r:id(p);v:*s",
        "E302",
    );
    rejects(
        "pick<&!int32>:(p<&!int32>,q<&!int32>){->p};x:=1;p:&!x;q:&!*p;r:pick(p,q)",
        "E302",
    );
    rejects(
        "pick<&int32>:(p<&!int32>,q<&int32>){->q};x:=1;p:&!x;s:&*p;r:pick(p,s)",
        "E302",
    );
}

#[test]
pub fn return_calls_capture_inputs_before_later_holder_replacement() {
    Case::new(
        r#"
d:@"debug"
id<&!int32>:(p<&!int32>,n<int32>){->p}
a:=1;b:=2;p:=&!a
r:id(p,{p=&!b;->0});*r=3;d.print(*r);d.print(*p)
"#,
    )
    .runs(b"3\n2\n");
}

#[test]
pub fn nonreturning_arguments_and_scoped_exits_do_not_create_results() {
    Case::new(
        r#"
d:@"debug"
id<&!int32>:(p<&!int32>,n<int32>){d.print(99);->p}
x:=1;p:&!x;'out{id(p,{x=2;d.print(x);'out.leave()})};d.print(x)
"#,
    )
    .runs(b"2\n2\n");
    rejects(
        "id<&!int32>:(p<&!int32>,n<int32>){->p};x:=1;p:&!x;'out{id(p,{'out.leave()})};v:*p",
        "E301",
    );
    let case = Case::new(
        r#"d:@"debug";die<&!int32>:(p<&!int32>){*p=2;d.print(*p);d.panic("stop")};x:=1;r:die(&!x)"#,
    );
    for profile in ["debug", "release"] {
        let output = case.command("run", &["--profile", profile]);
        assert!(!output.status.success());
        assert_eq!(output.stdout, b"2\n");
        assert!(String::from_utf8_lossy(&output.stderr).starts_with("panic[P006]: stop"));
    }
}

#[test]
pub fn wider_shapes_and_named_results_remain_gated() {
    for source in [
        "f<{p<&!int32>}>:(p<&!int32>){->p:p}",
        "f<&!int32>:(p<&!int32>,s<string>){->p}",
        "id<&!int32>:(p<&!int32>){->p};x:=1;p:&!x;r:{->view:id(p)}",
        "id<&int32>:(p<&int32>){->p};x:=1;p:&!x;r:{->view:id(p)}",
        "f<&!int32>:(p<&!int32>){'again{'again.restart()};->p}",
        "id<&!int32>:(p<&!int32>){->p};x:=1;r:id(&!x);s:&r",
        "id<&!int32>:(p<&!int32>){->p};x:=1;r:id(&!x);r.{v:*$}",
    ] {
        rejects(source, "B001");
    }
    rejects("bad<&!int32>:(p<&int32>){->&!*p}", "E305");
}

#[test]
pub fn returned_targets_are_captured_through_only_returning_stores() {
    Case::new(r#"d:@"debug";id<&!int32>:(p<&!int32>){d.print(1);->p};x:=2;p:&!x;*(id(&!*p))={d.print(3);->4};d.print(*p);'out{*(id(p))={x=5;'out.leave()}};d.print(x)"#).runs(b"1\n3\n4\n1\n5\n");
    rejects(
        "id<&!int32>:(p<&!int32>){->p};x:=1;p:&!x;*(id(&!*p))={*p=2;->3}",
        "E302",
    );
}

#[test]
pub fn return_parent_chains_execute_or_fail_with_bounded_proofs() {
    for (count, code) in [(16, None), (512, Some("B001"))] {
        let mut source = String::from("d:@\"debug\";id<&!int32>:(p<&!int32>){->p};x:=1;p0:&!x;");
        for index in 1..count {
            source.push_str(&format!("p{index}:id(p{});", index - 1));
        }
        source.push_str(&format!("d.print(*p{});", count - 1));
        if let Some(code) = code {
            rejects(&source, code);
        } else {
            Case::new(&source).runs(b"1\n");
        }
    }
}

#[test]
pub fn shared_return_calls_preserve_existing_restart_behavior() {
    Case::new(
        r#"
d:@"debug"
id<&int32>:(p<&int32>){->p}
x:=7;r:=&x;n:=0
'again{r=id(r);n=n+1;|n<3|'again.restart()}
d.print(*r);x=8;d.print(x)
"#,
    )
    .runs(b"7\n8\n");
}
