use super::Case;

#[test]
pub fn shared_reborrows_preserve_identity_and_end_after_final_use() {
    Case::new(
        r#"
d:@"debug"
owner:=1
parent:&owner
view:&*parent
nested:&*(&*view)
d.print(view==parent)
d.print(nested==&owner)
d.print(*nested)
owner=2
d.print(owner)
direct:&*(&owner)
d.print(*direct)
owner=3
d.print(owner)
"#,
    )
    .runs(b"true\ntrue\n1\n2\n2\n3\n");
}

#[test]
pub fn field_reborrows_use_original_nested_storage() {
    Case::new(
        r#"
d:@"debug"
<R>:<{small<uint8>;value<int64>;nested<{count<int32>}>}>
owner<R>:={->small:7;->value:41;->nested:{->count:9}}
parent:&owner
small:&(parent.small)
wide:&((*parent).value)
leaf:&((parent.nested).count)
d.print(small==&(owner.small))
d.print(wide==&(owner.value))
d.print(leaf==&(owner.nested.count))
d.print(*small)
d.print(*wide)
d.print(*leaf)
owner={->small:8;->value:42;->nested:{->count:10}}
d.print(owner.value)
"#,
    )
    .runs(b"true\ntrue\ntrue\n7\n41\n9\n42\n");
}

#[test]
pub fn reborrowed_function_fields_preserve_call_bounds_and_evaluate_once() {
    Case::new(
        r#"
d:@"debug"
<R>:<{value<int32>;nested<{other<int32>}>}>
get<&R>:(value<&R>){d.print("get");->value}
field<&int32>:(value<&R>){->&(value.nested.other)}
relay<&int32>:(value<&R>){->&(*(field(value)))}
owner<R>:={->value:11;->nested:{->other:22}}
view:&(get(&owner).value)
d.print(*view)
leaf:&((*(get(&owner))).nested.other)
d.print(*leaf)
d.print(*(relay(&owner)))
d.print((&(get(&owner).value))==&(owner.value))
d.print((&(*(get(&owner))))==&owner)
owner={->value:33;->nested:{->other:44}}
d.print(owner.value)
"#,
    )
    .runs(b"get\n11\nget\n22\n22\nget\ntrue\nget\ntrue\n33\n");
}

#[test]
pub fn guarded_reborrow_parents_keep_branch_and_iteration_origins() {
    Case::new(
        r#"
d:@"debug"
inspect<null>:(flag<boolean>){
    left:=11
    right:=22
    parent:{|flag|->&left;|!flag|->&right}
    view:&*parent
    |flag|right=33
    |!flag|left=44
    d.print(*view)
}
inspect(true)
inspect(false)
owner:=7
optional<&int32><null>:&owner
|optional<&int32>|{view:&(*(optional~<&int32>));d.print(*view)}
owner=8
count:=0
'loop {
    parent:&owner
    view:&*parent
    d.print(*view)
    owner=owner+1
    count=count+1
    |count<2|'loop.restart()
}
d.print(owner)
"#,
    )
    .runs(b"11\n22\n7\n8\n9\n10\n");
}

#[test]
pub fn shared_reborrow_uses_and_inherited_bounds_reject_live_writes() {
    for source in [
        "owner:=1;parent:&owner;view:&*parent;owner=2;value:*view",
        "owner:={->value:1};parent:&owner;view:&(parent.value);owner={->value:2};value:*view",
        "<R>:<{value<int32>}>;field<&int32>:(value<&R>){->&(value.value)};owner<R>:={->value:1};view:field(&owner);owner={->value:2};value:*view",
        "first<&int32>:(a<&int32>,b<&string>){->a};owner:=1;other:=\"old\";view:&(*(first(&owner,&other)));other=\"new\";value:*view",
        "owner:=1;parent:&owner;same:(&*parent)=={owner=2;->&owner}",
        "owner:=1;parent:&owner;result:{->&*parent;owner=2};value:*result",
        "owner:=1;parent:&owner;view:&*parent;i:=0;'loop{value:*view;owner=2;i=i+1;|i<2|'loop.restart()}",
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
pub fn shared_reborrow_escapes_and_remaining_storage_boundaries_are_explicit() {
    for (source, code) in [
        ("view:{owner:1;parent:&owner;->&*parent}", "E303"),
        (
            "view:{owner:{->value:1};parent:&owner;->&(parent.value)}",
            "E303",
        ),
        (
            "<R>:<{value<int32>}>;field<&int32>:(value<&R>){->&(value.value)};bad<&int32>:(){owner<R>:{->value:1};->field(&owner)}",
            "E303",
        ),
        (
            "first<&int32>:(a<&int32>,b<&string>){->a};owner:1;view:{short:\"local\";->&(*(first(&owner,&short)))}",
            "E303",
        ),
        ("owner:=1;parent:&owner;view:&!*parent", "E305"),
        (
            "make<{value<int32>}>:(){->value:1};view:&(make().value);copy:*view",
            "E303",
        ),
        ("view:&(({->value:1}).value);copy:*view", "E303"),
    ] {
        let result = Case::new(source).command("check", &["--json"]);
        assert_eq!(result.status.code(), Some(1), "{source}");
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(
            error.contains(&format!("\"code\":\"{code}\"")),
            "{source}: {error}"
        );
    }
}

#[test]
pub fn reborrow_hints_preserve_argument_leave_and_operand_effects() {
    Case::new(
        r#"
d:@"debug"
<R>:<{n<int32>}>
get<&R>:(p<&R>){d.print("get");->p}
inspect<int32>:(flag<boolean>) 'out {
    owner<R>:{->n:1}
    same:(&(get({|flag|{'out->7;'out.leave()};->&owner}).n))==&(owner.n)
    ->8
}
d.print(inspect(true))
d.print(inspect(false))
"#,
    )
    .runs(b"7\nget\n8\n");
}
