use super::Case;

#[test]
pub fn parameter_borrows_address_local_copies_and_preserve_argument_order() {
    Case::new(
        r#"
d:@"debug"
read<int32>:(value<int32>){view:{->&value};->*view}
same<boolean>:(value<int32>,original<&int32>){->&value==original}
first<int32>:(value<int32>,ignored<int32>){view:&value;->*view}
<R>:<{n<int32>;nested<{other<int64>}>}>
field<int64>:(value<R>){view:&(value.nested.other);->*view}
owner:=7
d.print(same(owner,&owner))
d.print(read(owner))
d.print(first(owner,{owner=8;->0}))
d.print(owner)
d.print(field({->n:1;->nested:{->other:9}}))
"#,
    )
    .runs(b"false\n7\n7\n8\n9\n");
}

#[test]
pub fn value_dispatch_borrows_its_copy_and_shared_dispatch_keeps_the_owner() {
    Case::new(
        r#"
d:@"debug"
owner:=4
copy:owner.{owner=5;view:&$;->*view}
d.print(copy)
d.print(owner)
<R>:<{n<int32>;nested<{other<int32>}>}>
record<R>:={->n:6;->nested:{->other:7}}
count:record.{view:&($.n);->*view}
d.print(count)
view:(&record).{->&($.nested.other)}
d.print(view==&(record.nested.other))
d.print(*view)
record={->n:8;->nested:{->other:9}}
d.print(record.n)
"#,
    )
    .runs(b"4\n5\n6\ntrue\n7\n8\n");
}

#[test]
pub fn reference_carrier_dispatch_preserves_components_and_nullable_activity() {
    Case::new(
        r#"
d:@"debug"
a:=11
b:=22
pair:{->left:&a;->right:&b}
view:pair.{->$.left}
b=23
d.print(*view)
a=12
inspect<null>:(flag<boolean>){
    owner:=7
    value<&int32><null>:{|flag|->&owner}
    result:value.{|$<&int32>|->$~<&int32>}
    |result<null>|{owner=8;d.print("none")}
    |result<&int32>|d.print(*(result~<&int32>))
    owner=9
    d.print(owner)
}
inspect(true)
inspect(false)
"#,
    )
    .runs(b"11\n7\n9\nnone\n9\n");
}

#[test]
pub fn borrowed_dispatch_operands_run_once_and_preserve_exits() {
    Case::new(
        r#"
d:@"debug"
get<&int32>:(value<&int32>){d.print("get");->value}
owner:=7
view:get(&owner).{->&*$}
d.print(*view)
owner=8
inspect<int32>:(flag<boolean>) 'out {
    value:3
    ->get({|flag|{'out->9;'out.leave()};->&value}).{view:&*$;->*view}
}
d.print(inspect(true))
d.print(inspect(false))
"#,
    )
    .runs(b"get\n7\n9\nget\n3\n");
}

#[test]
pub fn parameter_and_receiver_copy_addresses_cannot_escape() {
    for source in [
        "bad<&int32>:(value<int32>){->&value}",
        "bad:(value<int32>){view:{->&value};->view}",
        "<R>:<{n<int32>}>;bad<&int32>:(value<R>){->&(value.n)}",
        "owner:1;view:owner.{->&$}",
        "owner:{->n:1};view:owner.{->&($.n)}",
        "bad<&int32>:(value<int32>){->(&value).{->&*$}}",
        "first<&int32>:(a<&int32>,b<&string>){->a};owner:1;view:{short:\"x\";->first(&owner,&short).{->$}}",
    ] {
        let result = Case::new(source).command("check", &["--json"]);
        assert_eq!(result.status.code(), Some(1), "{source}");
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(error.contains("\"code\":\"E303\""), "{source}: {error}");
    }
}

#[test]
pub fn shared_dispatch_keeps_active_and_inherited_loans() {
    for source in [
        "owner:=1;value:(&owner).{owner=2;->*$}",
        "owner:=1;view:(&owner).{->$};owner=2;value:*view",
        "first<&int32>:(a<&int32>,b<&string>){->a};a:=1;b:=\"x\";view:first(&a,&b).{->&*$};b=\"y\";value:*view",
        "owner:=1;view:(&owner).{->&*$;owner=2};value:*view",
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
pub fn carrier_field_reborrows_copy_the_reference_prefix_once() {
    Case::new(
        r#"
d:@"debug"
<R>:<{n<int32>}>
<H>:<{view<&R>;count<int32>}>
make<H>:(p<&R>){d.print("make");->view:p;->count:3}
owner<R>:={->n:7}
holder<H>:{->view:&owner;->count:4}
view:holder.{->&($.view.n)}
d.print(view==&(owner.n))
d.print(*view)
other:&(make(&owner).view.n)
d.print(*other)
maybe<{view<&R><null>}>:{->view:&owner}
|maybe.view<&R>|{field:&(maybe.view.n);d.print(*field)}
owner={->n:8}
d.print(holder.count)
d.print(owner.n)
"#,
    )
    .runs(b"true\n7\nmake\n7\n7\n4\n8\n");
    for (source, code) in [
        (
            "view:{owner:{->n:1};holder:{->view:&owner};->&(holder.view.n)}",
            "E303",
        ),
        (
            "owner:={->n:1};holder:{->view:&owner};view:&(holder.view.n);owner={->n:2};read:*view",
            "E302",
        ),
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
pub fn receiver_sigil_keeps_nested_receivers_and_literal_dollars() {
    Case::new(
        r#"
d:@"debug"
value:7
answer:value.{outer:$;inner:3.{->$*2};->{->outer:outer;->inner:inner;->again:$}}
d.print("{answer.outer} {answer.inner} {answer.again}")
d.print(4.{->{->$+1}})
9.{d.print("receiver={$}; literal=$")}
"#,
    )
    .runs(b"7 6 7\n5\nreceiver=9; literal=$\n");
}

#[test]
pub fn receiver_sigil_keeps_borrows_and_mutation_permissions() {
    Case::new(
        r#"
d:@"debug"
owner:=7
copy:owner.{owner=9;view:&$;->*view}
d.print("{copy} {owner}")
row:={->n:=1}
result:row.{$.n=5;->$.n}
d.print("{result} {row.n}")
"#,
    )
    .runs(b"7 9\n5 1\n");
    for source in [
        "owner:1;view:owner.{->&$}",
        "owner:=1;view:(&owner).{->&*$};owner=2;out:*view",
    ] {
        let code = if source.starts_with("owner:1") {
            "E303"
        } else {
            "E302"
        };
        let result = Case::new(source).command("check", &["--json"]);
        assert_eq!(result.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&result.stderr).contains(code));
    }
}

#[test]
pub fn receiver_sigil_is_independent_of_ordinary_self_names() {
    Case::new(
        r#"
d:@"debug"
self:9
d.print(3.{->$+self})
d.print(3.{self:4;->$+self})
identity<int32>:(self<int32>){->self}
d.print(identity(5))
"#,
    )
    .runs(b"12\n7\n5\n");
}
