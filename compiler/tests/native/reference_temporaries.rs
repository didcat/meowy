use super::Case;

#[test]
pub fn temporary_reference_cells_keep_original_pointees_and_distinct_addresses() {
    Case::new(
        r#"
d:@"debug"
<R>:<&int32>
same<boolean>:(a<&R>,b<&R>){->a==b}
owner:=7
copied:*(&(&owner))
d.print(copied==&owner);d.print(*copied)
d.print(same(&(&owner),&(&owner)))
d.print(**(&(&owner)))
owner=8
d.print(owner)
"#,
    )
    .runs(b"true\n7\nfalse\n7\n8\n");
}

#[test]
pub fn temporary_carrier_copies_and_field_reborrows_preserve_external_owners() {
    Case::new(
        r#"
d:@"debug"
<H>:<{view<&int32>;count<int32>}>
make<H>:(p<&int32>){d.print("make");->view:p;->count:3}
owner:7
copy:*(&(make(&owner)))
d.print(*(copy.view));d.print(copy.count)
direct:*(&{->view:&owner;->count:4})
d.print(*(direct.view))
copied:*(&(make(&owner).view))
d.print(copied==&owner)
row:{->n:9}
field:&({->view:&row}.view.n)
d.print(field==&(row.n));d.print(*field)
d.print(*(&(make(&owner).count)))
"#,
    )
    .runs(b"make\n7\n3\n7\nmake\ntrue\ntrue\n9\nmake\n3\n");
}

#[test]
pub fn temporary_carrier_union_snapshots_preserve_nullable_tags() {
    Case::new(
        r#"
d:@"debug"
<H>:<{view<&int32><null>;count<int32>}>
make<H>:(p<&int32>,present<boolean>){|present|->view:p;->count:2}
owner:=7
empty:*(&(make(&owner,false)))
full:*(&(make(&owner,true)))
|empty.view<null>|d.print("empty")
|full.view<&int32>|d.print(*(full.view~<&int32>))
owner=8
d.print(empty.count)
none:*(&{->view<&int32><null>:null})
|none.view<null>|d.print("null")
"#,
    )
    .runs(b"empty\n7\n2\nnull\n");
}

#[test]
pub fn temporary_materialization_reads_values_but_defers_nested_pointees() {
    Case::new(
        r#"
d:@"debug"
owner:=1
original:&owner
outer:&original
owner=2
copy:*(&outer)
d.print(copy==outer)
expired:&3
pointer:*(&(&expired))
d.print(pointer==&expired)
"#,
    )
    .runs(b"true\ntrue\n");
    for source in [
        "owner:=1;view:&owner;owner=2;copy:*(&view)",
        "owner:=1;holder:{->view:&owner;->count:3};owner=2;value:*(&({->holder}.count))",
        "owner:=1;view:&owner;outer:&view;owner=2;copied:*(&outer);value:**copied",
    ] {
        let output = Case::new(source).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E302\""));
    }
}

#[test]
pub fn reference_temporary_cell_expiry_does_not_extend_contained_temporary_owners() {
    for source in [
        "owner:1;cell:&(&owner);value:**cell",
        "owner:1;pointer:&{->view:&owner};value:*(pointer.view)",
        "owner:1;field:&({->view:&owner;->count:3}.count);value:*field",
        "copied:*(&(&1));value:*copied",
        "copied:*(&{->view:&1})",
        "copied:*({->&(&1)})",
        "owner:1;value:{->&(&owner)}",
    ] {
        let output = Case::new(source).command("build", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E303\""));
    }
}

#[test]
pub fn reference_temporary_calls_retain_public_bounds_after_dereference() {
    Case::new(
        r#"
d:@"debug"
<R>:<&int32>
load<R>:(p<&R>){->*p}
<H>:<{view<R>}>
copy<H>:(p<&H>){->*p}
identity<R>:(p<R>){->p}
owner:7
d.print(*(load(&(&owner))))
d.print(*(copy(&{->view:&owner}).view))
direct:*(&(identity(&owner)))
d.print(*direct)
"#,
    )
    .runs(b"7\n7\n7\n");
    for source in [
        "<R>:<&int32>;load<R>:(p<&R>){->*p};owner:1;view:load(&(&owner));value:*view",
        "<H>:<{view<&int32>}>;copy<H>:(p<&H>){->*p};owner:1;value:copy(&{->view:&owner});read:*(value.view)",
        "first<&int32>:(p<&int32>,q<&string>){->p};owner:1;view:*(&(first(&owner,&\"short\")));value:*view",
        "<R>:<&int32>;load<int32>:(p<&R>){->**p};bad:*(&(&1));value:load(&bad)",
    ] {
        let output = Case::new(source).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E303\""));
    }
}

#[test]
pub fn reference_temporary_effects_and_iteration_snapshots_evaluate_once() {
    Case::new(
        r#"
d:@"debug"
<R>:<&int32>
identity<R>:(p<R>){d.print("reference");->p}
owner:7
d.print(**(&(identity(&owner))))
count:=0
'loop{
    copied:*(&{->view:&owner;->count:count})
    d.print(copied.count)
    count=count+1
    |count<2|'loop.restart()
}
'out{unused:&{d.print("leave");'out.leave();->&owner}}
"#,
    )
    .runs(b"reference\n7\n0\n1\nleave\n");
    let source = "d:@\"debug\";owner:1;unused:&{d.print(\"owner\");d.panic(\"stop\");->&owner}";
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
pub fn reference_temporary_mutation_and_owned_collection_boundaries_stay_explicit() {
    Case::new("owner:1;value:={->view:&owner}").runs(b"");
    Case::new("owner:1;view<&int32><null>:=&owner").runs(b"");
    for source in [
        "owner:1;items:[&owner]",
        "owner:1;n:=2;holder:'out{'loop{'out->view:=&owner;view=&owner;n=n-1;|n>0|'loop.restart()}}",
    ] {
        let output = Case::new(source).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"B001\""));
    }
}
