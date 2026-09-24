use super::Case;
use super::exclusive_references::rejects;

#[test]
pub fn scalar_parameters_mutate_actual_owners_in_both_profiles() {
    Case::new(
        r#"
d:@"debug"
bump<int32>:(p<&!int32>){*p=*p+1;->*p}
flip<boolean>:(p<&!boolean>){*p=!*p;->*p}
small<int8>:(p<&!int8>){*p=*p+1;->*p}
float<float32>:(p<&!float32>){*p=*p+0.5;->*p}
x:=1;d.print(bump(&!x));d.print(x)
b:=true;d.print(flip(&!b));d.print(b)
i<int8>:=126;d.print(small(&!i));d.print(i)
f<float32>:=1.5;d.print(float(&!f));d.print(f)
"#,
    )
    .runs(b"2\n2\nfalse\nfalse\n127\n127\n2\n2\n");
}

#[test]
pub fn moved_and_reborrowed_arguments_keep_distinct_holder_lifetimes() {
    Case::new(
        r#"
d:@"debug"
set<null>:(p<&!int32>,n<int32>){*p=n}
x:=1;p:&!x;set(p,2);d.print(x)
a:=3;q:&!a;set(&!*q,4);d.print(*q);set(&!*q,5);d.print(*q)
b:=6;r:=&!b;set(r,7);r=&!b;set(r,8);d.print(b)
"#,
    )
    .runs(b"2\n4\n5\n8\n");
}

#[test]
pub fn shared_arguments_reborrow_without_consuming_the_parent() {
    Case::new(
        r#"
d:@"debug"
read<int32>:(p<&int32>){->*p}
pair<int32>:(p<&int32>,q<&int32>){->*p+*q}
x:=1;p:&!x;s:&*p;t:s;d.print(read(t));d.print(read(p));d.print(pair(p,p));*p=2;d.print(*p)
"#,
    )
    .runs(b"1\n1\n2\n2\n");
}

#[test]
pub fn nested_calls_preserve_symbolic_parent_authority() {
    Case::new(
        r#"
d:@"debug"
read<int32>:(p<&int32>){->*p}
bump<null>:(p<&!int32>){*p=*p+1}
outer<int32>:(p<&!int32>){s:&*p;v:read(s);bump(&!*p);q:p;bump(q);->v}
x:=1;d.print(outer(&!x));d.print(x)
"#,
    )
    .runs(b"1\n3\n");
}

#[test]
pub fn recursion_and_function_aliases_revalidate_each_call() {
    Case::new(
        r#"
d:@"debug"
count<null>:(p<&!int32>,n<int32>){|n>0|{*p=*p+1;count(&!*p,n-1)}}
run:count
x:=1;p:&!x;run(&!*p,4);d.print(*p)
"#,
    )
    .runs(b"5\n");
}

#[test]
pub fn distinct_input_roots_allow_exclusive_access() {
    Case::new(
        r#"
d:@"debug"
swap<null>:(p<&!int32>,q<&!int32>){v:*p;*p=*q;*q=v}
add<null>:(p<&!int32>,q<&int32>){*p=*p+*q}
a:=1;b:=2;swap(&!a,&!b);add(&!a,&b);d.print(a);d.print(b)
"#,
    )
    .runs(b"3\n1\n");
}

#[test]
pub fn guarded_argument_choices_and_conditional_moves_preserve_paths() {
    Case::new(
        r#"
d:@"debug"
set<null>:(p<&!int32>,q<&int32>){*p=*q}
f<null>:(flag<boolean>){
 a:=1;b:=2;p:=&!a
 |flag|p=&!b
 |flag|set(p,&a)
 |!flag|set(p,&b)
 d.print(a);d.print(b)
}
f(true);f(false)
move<null>:(flag<boolean>){a:=3;p:&!a;|flag|set(p,&4);|!flag|d.print(*p);d.print(a)}
move(true);move(false)
"#,
    )
    .runs(b"1\n1\n2\n2\n4\n3\n3\n");
}

#[test]
pub fn later_argument_leave_skips_call_but_preserves_earlier_moves() {
    Case::new(
        r#"
d:@"debug"
set<null>:(p<&!int32>,n<int32>){d.print(99);*p=n}
x:=1;p:&!x
'out{set(p,{x=2;d.print(x);'out.leave()})}
d.print(x)
"#,
    )
    .runs(b"2\n2\n");
    rejects(
        "set<null>:(p<&!int32>,n<int32>){*p=n};x:=1;p:&!x;'out{set(p,{'out.leave()})};v:*p",
        "E301",
    );
}

#[test]
pub fn moved_parameters_and_call_arguments_report_exact_codes() {
    for source in [
        "set<null>:(p<&!int32>){*p=2};x:=1;p:&!x;set(p);v:*p",
        "f<int32>:(p<&!int32>){q:p;->*p}",
        "set<null>:(p<&!int32>){*p=2};f<null>:(p<&!int32>){set(p);set(p)}",
        "f<null>:(a<&!int32>,b<&!int32>){};x:=1;p:&!x;f(p,p)",
    ] {
        rejects(source, "E301");
    }
    rejects(
        "set<null>:(p<&!int32>){};f<null>:(flag<boolean>){x:=1;p:&!x;|flag|set(p);v:*p}",
        "E309",
    );
    rejects(
        "f<null>:(p<&!int32>,flag<boolean>){|flag|{q:p};v:*p}",
        "E309",
    );
}

#[test]
pub fn call_entry_checks_suspended_parents_and_all_arguments() {
    for source in [
        "set<null>:(p<&!int32>){*p=2};x:=1;p:&!x;s:&*p;set(p);v:*s",
        "set<null>:(p<&!int32>){};x:=1;p:&!x;s:&*p;t:s;set(p);v:*t",
        "f<null>:(p<&!int32>,s<&int32>){*p=*s};x:=1;p:&!x;s:&*p;f(p,s)",
        "f<null>:(s<&int32>,p<&!int32>){};x:=1;p:&!x;s:&*p;f(s,p)",
        "f<null>:(p<&!int32>,q<&!int32>){};x:=1;p:&!x;q:&!*p;f(p,q)",
        "read<int32>:(p<&int32>){->*p};x:=1;p:&!x;q:&!*p;v:read(p);w:*q",
        "set<null>:(p<&!int32>,n<int32>){*p=n};x:=1;p:&!x;set(&!*p,{v:*p;->2})",
        "set<null>:(p<&!int32>,n<int32>){*p=n};x:=1;set(&!x,{x=2;->3})",
    ] {
        rejects(source, "E302");
    }
}

#[test]
pub fn symbolic_input_conflicts_survive_reborrows_and_forwarding() {
    for source in [
        "f<null>:(p<&!int32>){s:&*p;*p=2;v:*s}",
        "f<null>:(p<&!int32>){q:&!*p;v:*p;w:*q}",
        "set<null>:(p<&!int32>){};f<null>:(p<&!int32>){s:&*p;set(p);v:*s}",
        "set<null>:(p<&!int32>,q<&int32>){};f<null>:(p<&!int32>){s:&*p;set(p,s)}",
        "f<null>:(p<&!int32>,flag<boolean>){q:=&!*p;|flag|{s:&*q;*q=2;v:*s}}",
    ] {
        rejects(source, "E302");
    }
}

#[test]
pub fn opaque_public_bounds_still_protect_call_dependencies() {
    rejects(
        "first<&int32>:(a<&int32>,b<&int32>){->a};set<null>:(p<&!int32>){};a:=1;b:=2;r:first(&a,&b);set(&!b);v:*r",
        "E302",
    );
    rejects(
        "first<&int32>:(a<&int32>,b<string>){->a};f<null>:(p<&!int32>){r:first(p,\"\");v:*r}",
        "B001",
    );
}

#[test]
pub fn excluded_result_shapes_and_restart_inputs_remain_gated() {
    for source in [
        "f<{p<&!int32>}>:(p<&!int32>){->p:p}",
        "f<&int32>:(p<&!int32>,s<string>){->&*p}",
        "f<{n<int32>}>:(p<&!int32>){->n:*p}",
        "f<null>:(p<&!int32>,s<string>){}",
        "f<null>:(p<&!int32>){'again{'again.restart()}}",
        "read<{n<int32>}>:(p<&int32>){->n:*p};x:=1;p:&!x;v:read(p)",
        "f<null>:(p<&!int32>){v:p.{->*$}}",
        "f<null>:(p<&!int32>){s:&*p;cell:&s}",
    ] {
        rejects(source, "B001");
    }
    rejects("f<null>:(p<&int32>){*p=2}", "E305");
}

#[test]
pub fn no_return_calls_still_validate_entry_and_argument_effects() {
    let case = Case::new(
        r#"d:@"debug";die<never>:(p<&!int32>){*p=2;d.print(*p);d.panic("stop")};x:=1;die(&!x)"#,
    );
    for profile in ["debug", "release"] {
        let output = case.command("run", &["--profile", profile]);
        assert!(!output.status.success());
        assert_eq!(output.stdout, b"2\n");
        assert!(String::from_utf8_lossy(&output.stderr).starts_with("panic[P006]: stop"));
    }
    rejects(
        "d:@\"debug\";die<never>:(p<&!int32>,s<&int32>){d.panic(\"stop\")};x:=1;p:&!x;s:&*p;die(p,s)",
        "E302",
    );
    let case = Case::new(
        r#"d:@"debug";set<null>:(p<&!int32>,n<int32>){d.print(99);*p=n};x:=1;p:&!x;set(p,{x=2;d.print(x);d.panic("argument")})"#,
    );
    for profile in ["debug", "release"] {
        let output = case.command("run", &["--profile", profile]);
        assert!(!output.status.success());
        assert_eq!(output.stdout, b"2\n");
        assert!(String::from_utf8_lossy(&output.stderr).starts_with("panic[P006]: argument"));
    }
}

#[test]
pub fn receiver_syntax_for_direct_calls_uses_the_same_argument_contract() {
    Case::new(r#"d:@"debug";bump<null>:(p<&!int32>){*p=*p+1};read<int32>:(p<&int32>){->*p};x:=1;p:&!x;(&!*p).(bump);d.print(p.(read));*p=3;d.print(*p)"#).runs(b"2\n3\n");
    rejects(
        "bump<null>:(p<&!int32>){};x:=1;p:&!x;s:&*p;p.(bump);v:*s",
        "E302",
    );
}

#[test]
pub fn mutating_calls_invalidate_branch_and_index_refinements() {
    Case::new(
        r#"
d:@"debug"
flip<null>:(p<&!boolean>){*p=!*p}
set<null>:(p<&!int32>){*p=2}
b:=true;|b|d.print(1);flip(&!b);|b|d.print(99);|!b|d.print(2)
n:=1;set(&!n);items:[7,8];d.print(items[n])
"#,
    )
    .runs(b"1\n2\n8\n");
}

#[test]
pub fn shared_only_restart_callees_keep_readonly_input_support() {
    Case::new(
        r#"
d:@"debug"
read<int32>:(p<&int32>,count<int32>) 'out {
 n:=count
 'again{|n>0|{n=n-1;'again.restart()}}
 ->*p
}
x:=1;p:&!x;d.print(read(p,3));*p=2;d.print(*p)
"#,
    )
    .runs(b"1\n2\n");
}
