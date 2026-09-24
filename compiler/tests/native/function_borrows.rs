use super::Case;

#[test]
pub fn function_borrows_preserve_pointer_identity_and_final_use() {
    Case::new(
        r#"
d:@"debug"
identity<&int32>:(value<&int32>){->value}
relay<&int32>:(value<&int32>){->identity(value)}
inferred:(value<&int32>){->relay(value)}
read<int32>:(value<&int32>){->*value}
alias:relay
owner:=41
view:alias(&owner)
d.print(view==&owner)
d.print(read(view))
owner=42
owner=read(&owner)+1
d.print(owner)
d.print(*(inferred(&owner)))
record:{->value:7}
field:identity(&(record.value))
d.print(*field)
"#,
    )
    .runs(b"true\n41\n43\n43\n7\n");
}

#[test]
pub fn function_carriers_keep_temporary_inputs_and_returned_components() {
    Case::new(
        r#"
d:@"debug"
<R>:<{view<&int32>;nested<{other<&string>}>;count<int32>}>
copy<R>:(value<R>){->value}
head<&int32>:(value<{view<&int32>}>){->value.view}
owner:=17
text:="word"
record:copy({->view:&owner;->nested:{->other:&text};->count:3})
d.print(*(record.view))
d.print(*(record.nested.other))
owner=18
text="changed"
d.print(record.count)
view:head({->view:&owner})
d.print(*view)
owner=19
d.print(owner)
"#,
    )
    .runs(b"17\nword\n3\n18\n19\n");
}

#[test]
pub fn function_contracts_ignore_absent_inputs_and_scalar_only_results() {
    Case::new(
        r#"
d:@"debug"
first<&int32>:(value<&int32>,ignored<&string><null>){->value}
measure<int32>:(value<&int32>,ignored<&string>){->*value}
packet<{view<&int32>;count<int32>}>:(value<&int32>,ignored<&string>){->view:value;->count:*value}
owner:=5
text:="old"
missing<&string><null>:null
view:first(&owner,missing)
text="new"
d.print(*view)
owner=6
number:measure(&owner,&text)
owner=7
text="next"
d.print(number)
record:packet(&owner,&text)
owner=8
text="last"
d.print(record.count)
d.print(owner)
"#,
    )
    .runs(b"5\n6\n7\n8\n");
}

#[test]
pub fn recursive_function_borrows_use_declared_contracts() {
    Case::new(
        r#"
d:@"debug"
walk<&int32>:(value<&int32>,depth<int32>) 'result {
    |depth<=0|{'result->value;'result.leave()}
    ->walk(value,depth-1)
}
owner:=9
view:walk(&owner,4)
d.print(*view)
owner=10
d.print(owner)
first<(&int32,int32)->&int32>;
second<(&int32,int32)->&int32>;
first<&int32>:(value<&int32>,depth<int32>) 'done {
    |depth<=0|{'done->value;'done.leave()}
    ->second(value,depth-1)
}
second<&int32>:(value<&int32>,depth<int32>) 'done {
    |depth<=0|{'done->value;'done.leave()}
    ->first(value,depth-1)
}
d.print(*(first(&owner,3)))
owner=11
d.print(owner)
"#,
    )
    .runs(b"9\n10\n10\n11\n");
}

#[test]
pub fn returned_views_retain_every_active_input_loan() {
    for source in [
        "first<&int32>:(a<&int32>,b<&int32>){->a};a:=1;b:=2;r:first(&a,&b);b=3;value:*r",
        "first<&int32>:(a<&int32>,b<&string>){->a};a:=1;b:=\"old\";r:first(&a,&b);b=\"new\";value:*r",
        "identity<&int32>:(a<&int32>){->a};first<&int32>:(a<&int32>,b<&string>){->a};a:=1;b:=\"old\";r:identity(first(&a,&b));b=\"new\";value:*r",
        "head<&int32>:(p<{left<&int32>;right<&int32>}>){->p.left};a:=1;b:=2;r:head({->left:&a;->right:&b});b=3;value:*r",
        "pair<{left<&int32>;right<&int32>}>:(a<&int32>,b<&int32>){->left:a;->right:b};a:=1;b:=2;r:pair(&a,&b);b=3;value:*(r.left)",
        "identity<&int32>:(a<&int32>){->a};a:=1;r:identity(&a);a=2;value:*r",
        "read<int32>:(a<&int32>,b<int32>){->*a+b};a:=1;x:read(&a,{a=2;->3})",
        "identity<&int32>:(a<&int32>){->a};a:=1;r:{->identity(&a);a=2};value:*r",
        "identity<&int32>:(a<&int32>){->a};a:=1;r:identity(&a);i:=0;'loop{value:*r;a=2;i=i+1;|i<2|'loop.restart()}",
    ] {
        let case = Case::new(source);
        for profile in ["debug", "release"] {
            let result = case.command("build", &["--json", "--profile", profile]);
            assert_eq!(result.status.code(), Some(1), "{source}");
            let error = String::from_utf8_lossy(&result.stderr);
            assert!(error.contains("\"code\":\"E302\""), "{source}: {error}");
            assert!(!case.path.join("build").exists());
        }
    }
}

#[test]
pub fn function_return_contracts_reject_local_and_ignored_input_escapes() {
    for source in [
        "bad<&int32>:(){owner:1;->&owner}",
        "bad<&int32>:(value<&int32>){owner:1;->&owner}",
        "identity<&int32>:(value<&int32>){->value};bad<&int32>:(){owner:1;->identity(&owner)}",
        "first<&int32>:(a<&int32>,b<&string>){->a};owner:1;r:{short:\"local\";->first(&owner,&short)}",
        "pair<{view<&int32>}>:(a<&int32>,b<&string>){->view:a};owner:1;r:{short:\"local\";->pair(&owner,&short)}",
        "first<&int32>:(a<&int32>,b<&int32>){->a};wrapper<&int32>:(a<&int32>){short:1;->first(a,&short)}",
    ] {
        let result = Case::new(source).command("check", &["--json"]);
        assert_eq!(result.status.code(), Some(1), "{source}");
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(error.contains("\"code\":\"E303\""), "{source}: {error}");
    }
}

#[test]
pub fn nullable_function_results_keep_active_bounds_and_empty_returns() {
    Case::new(
        r#"
d:@"debug"
<Maybe>:<&int32><null>
copy<Maybe>:(value<Maybe>){->value}
optional<Maybe>:(flag<boolean>,value<&int32>){|flag|->value;|!flag|->null}
empty<Maybe>:(){->null}
inspect<null>:(flag<boolean>){
    owner:=11
    view:copy(optional(flag,&owner))
    |view<null>|{owner=12;d.print("absent")}
    |view<&int32>|d.print(*(view~<&int32>))
    owner=13
    d.print(owner)
}
inspect(true)
inspect(false)
none:empty()
|none<null>|d.print("empty")
number:7
text:"text"
<AnyRef>:<&int32><&string>
keep<AnyRef>:(value<AnyRef>){->value}
a:keep(&number)
b:keep(&text)
|a<&int32>|d.print(*(a~<&int32>))
|b<&string>|d.print(*(b~<&string>))
"#,
    )
    .runs(b"11\n13\nabsent\n13\nempty\n7\ntext\n");
}

#[test]
pub fn separate_calls_keep_independent_bounds_and_local_scalar_use() {
    Case::new(
        r#"
d:@"debug"
identity<&int32>:(value<&int32>){->value}
first<&int32>:(a<&int32>,b<&string>){->a}
read<int32>:(a<&int32>){local:"short";->*(first(a,&local))}
left:=5
right:=6
pair:{->left:identity(&left);->right:identity(&right)}
right=7
d.print(*(pair.left))
left=8
d.print(read(&left))
"#,
    )
    .runs(b"5\n8\n");
}
