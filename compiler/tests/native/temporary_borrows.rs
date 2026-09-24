use super::Case;

#[test]
pub fn temporary_borrows_evaluate_once_and_keep_distinct_live_cells() {
    Case::new(
        r#"
d:@"debug"
make<int32>:(n<int32>){d.print(n);->n}
sum<int32>:(a<&int32>,b<&int32>){d.print("call");->*a+*b}
same<boolean>:(a<&int32>,b<&int32>){->a==b}
text<boolean>:(a<&string>,b<&string>){->a==b}
d.print(sum(&(make(2)),&(make(3))))
d.print(same(&7,&7));d.print(text(&"x",&"x"))
copy:*(&7)
d.print(copy);d.print(*(&"value"))
"#,
    )
    .runs(b"2\n3\ncall\n5\nfalse\nfalse\n7\nvalue\n");
}

#[test]
pub fn temporary_record_and_list_projections_use_the_materialized_owner() {
    Case::new(
        r#"
d:@"debug"
<R>:<{value<int32>;items<int32[3]>}>
make<R>:(){d.print("owner");->value:4;->items:[5,6]}
d.print(*(&(make().value)))
d.print(*(&(make().items[2])))
d.print(*(&([7,8][2])))
d.print((&(make())).{->*(&($.value))})
id<&int32>:(p<&int32>){->p}
owner:=9
view:&(*(id(&owner)))
d.print(*view)
owner=10
d.print(owner)
"#,
    )
    .runs(b"owner\n4\nowner\n6\n8\nowner\n4\n9\n10\n");
}

#[test]
pub fn temporary_references_survive_same_statement_calls_dispatch_and_matchers() {
    Case::new(
        r#"
d:@"debug"
id<&int32>:(p<&int32>){->p}
positive<boolean>:(p<&int32>){d.print("condition");->*p>0}
d.print(*(id(&(1+2))))
d.print(*((&{->n:4}).{->&($.n)}))
|positive(&5)|d.print("body")
|*(&true)|d.print("true")
copy:(&6).{->*$}
d.print(copy)
"#,
    )
    .runs(b"3\n4\ncondition\nbody\ntrue\n6\n");
}

#[test]
pub fn temporary_owners_end_at_their_actual_complete_statement() {
    for source in [
        "p:&1;v:*p",
        "id<&int32>:(p<&int32>){->p};view:id(&(1+2));v:*view",
        "p:&({->n:1}.n);v:*p",
        "p:&([1,2][1]);v:*p",
        "value:{->&1}",
        "value:*({->&{->1}})",
        "value:*({p:&1;->p})",
        "f<&int32>:(){->&1}",
        "p:(&{->n:1}).{->&($.n)};v:*p",
        "v:({->n:1}).{->&($.n)}",
        "read<int32>:(p<& &int32>){->**p};cell:&1;value:read(&cell)",
    ] {
        let case = Case::new(source);
        for profile in ["debug", "release"] {
            let output = case.command("build", &["--json", "--profile", profile]);
            assert_eq!(output.status.code(), Some(1), "{source}");
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains("\"code\":\"E303\""), "{source}: {error}");
            assert!(!case.path.join("build").exists());
        }
    }
    Case::new("d:@\"debug\";value:*(&{->1});d.print(value);p:&2;|false|v:*p").runs(b"1\n");
}

#[test]
pub fn temporary_call_bounds_preserve_original_owner_conflicts() {
    Case::new(
        r#"
d:@"debug"
read<int32>:(p<&int32>,other<&string>){->*p}
first<&int32>:(p<&int32>,other<&string>){->p}
owner:=1
d.print(read(&owner,&"short"))
owner=2
d.print(*(first(&owner,&"short")))
owner=3
d.print(owner)
"#,
    )
    .runs(b"1\n2\n3\n");
    for (source, code) in [
        (
            "first<&int32>:(p<&int32>,other<&string>){->p};owner:1;view:first(&owner,&\"short\");v:*view",
            "E303",
        ),
        (
            "read<int32>:(a<&int32>,b<&int32>){->*a+*b};owner:=1;v:read(&owner,&{owner=2;->3})",
            "E302",
        ),
    ] {
        let output = Case::new(source).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        assert!(String::from_utf8_lossy(&output.stderr).contains(&format!("\"code\":\"{code}\"")));
    }
}

#[test]
pub fn temporary_materialization_skips_nonreturning_owners_and_restarts_cleanly() {
    Case::new(
        r#"
d:@"debug"
value:'out{->*(&{d.print("leave");'out->7;'out.leave()})}
d.print(value)
i:=0
'loop{
    d.print(*(&{i=i+1;->i}))
    |i<2|'loop.restart()
}
'skip{unused:&([1][{d.print("index");'skip.leave();->1}]);d.print("unreachable")}
"#,
    )
    .runs(b"leave\n7\n1\n2\nindex\n");
    let source = "d:@\"debug\";value:*(&{d.print(\"owner\");d.panic(\"stop\")})";
    let call = "d.panic(\"stop\")";
    let start = source.find(call).unwrap();
    let end = start + call.len();
    let case = Case::new(source);
    for profile in ["debug", "release"] {
        let output = case.command("run", &["--profile", profile]);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, b"owner\n");
        assert_eq!(
            output.stderr,
            format!("panic[P006]: stop at bytes {start}..{end}\n").as_bytes()
        );
    }
}

#[test]
pub fn temporary_list_bounds_keep_static_checks_and_dynamic_effect_order() {
    for source in ["p:&([1][0])", "p:&([1][2])"] {
        let output = Case::new(source).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E101\""));
    }
    let source = "d:@\"debug\";make<int32[3]>:(){d.print(\"owner\");->[1]};index:=2;value:*(&(make()[{d.print(\"index\");->index}]))";
    let access = "&(make()[{d.print(\"index\");->index}])";
    let start = source.find(access).unwrap();
    let end = start + access.len();
    let case = Case::new(source);
    for profile in ["debug", "release"] {
        let output = case.command("run", &["--profile", profile]);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, b"owner\nindex\n");
        assert_eq!(
            output.stderr,
            format!(
                "panic[P001]: index 2 is outside initialized length 1 at bytes {start}..{end}\n"
            )
            .as_bytes()
        );
    }
}

#[test]
pub fn temporary_borrow_operand_types_and_unsupported_owners_stay_explicit() {
    Case::new("d:@\"debug\";take<uint8>:(p<&uint8>){->*p};byte<uint8>:1;d.print(take(&(byte+0)))")
        .runs(b"1\n");
    for (source, code) in [
        ("take<uint8>:(p<&uint8>){->*p};v:take(&1)", "E212"),
        ("owner:1;p:&{->view:&owner};v:*(p.view)", "E303"),
        (
            "owner:1;id<&int32>:(p<&int32>){->p};cell:&(id(&owner));v:**cell",
            "E303",
        ),
        ("p:&!1", "B001"),
    ] {
        let output = Case::new(source).command("build", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        assert!(String::from_utf8_lossy(&output.stderr).contains(&format!("\"code\":\"{code}\"")));
    }
}

#[test]
pub fn temporary_payload_tags_do_not_read_expired_reference_values() {
    Case::new(
        r#"
d:@"debug"
holder:(&1).{->ref<&int32><null>:$;->count:2}
outer:&holder
|outer.ref<&int32>|d.print("reference")
next:&outer
|(*next).ref<&int32>|d.print("nested")
d.print(outer.count)
view:&3
|view<&int32>|d.print("type")
"#,
    )
    .runs(b"reference\nnested\n2\ntype\n");
    for source in [
        "view:&{->tag<int32><null>:null};|view.tag<int32>|value:1",
        "holder:(&1).{->ref:$};|({copy:holder.ref;->copy})<&int32>|value:1",
    ] {
        let output = Case::new(source).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E303\""));
    }
}
