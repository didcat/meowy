use super::*;

#[test]
pub fn mutable_fields_preserve_copies_neighbors_and_nested_values() {
    Case::new(
        r#"
d:@"debug"
<Inner>:<{left<int32>:=;right<int32>:=}>
<Outer>:<{inner<Inner>:=;label<string>}>
value<Outer>:={->inner:={->left:=1;->right:=2};->label:"kept"}
copy:value
value.inner.left=7
d.print(value.inner.left);d.print(value.inner.right);d.print(value.label)
d.print(copy.inner.left)
holder:={->items<int32[3]>:=[1];->choice<int32><string>:=1}
holder.items=[2,3]
holder.choice="changed"
d.print(holder.items.size());d.print(holder.items[2])
choice:holder.choice
|choice<string>|d.print(choice~<string>)
<Optional>:<{n<int32><null>:=}>
optional<Optional>:={}
initial:optional.n
|initial<null>|d.print("empty")
optional.n=42
filled:optional.n
|filled<int32>|d.print(filled~<int32>)
"#,
    )
    .runs(b"7\n2\nkept\n1\n2\n3\nchanged\nempty\n42\n");
}

#[test]
pub fn mutable_field_shapes_survive_calls_composition_and_list_contexts() {
    Case::new(
        r#"
d:@"debug"
<Read>:<{n<int32>}>
<Write>:<{n<int32>:=}>
make<Write>:(flag<boolean>){|flag|->n:=1;|!flag|->n:=2}
value:=make(true)
value.n=3
tag<Read><Write>:value
|tag<Write>|d.print(tag.n)
|tag<Read>|d.print("unreachable")
part:{->n:=4}
composed:={->part;->extra:=5}
composed.n=6
composed.extra=7
d.print(composed.n);d.print(composed.extra);d.print(part.n)
rows<Read[1]><Write[1]>:[{->n:=8}]
|rows<Write[1]>|d.print(rows[1].n)
fixed:make(false)
copy:=fixed
copy.n=9
d.print(fixed.n);d.print(copy.n)
<Narrow>:<{n<uint8>:=}>
<Wide>:<{n<uint16>}>
writable<Narrow><Wide>:{->n:=1}
readonly<Narrow><Wide>:{->n:1}
|writable<Narrow>|d.print("narrow")
|readonly<Wide>|d.print("wide")
"#,
    )
    .runs(b"3\n6\n7\n4\n8\n2\n9\nnarrow\nwide\n");
}

#[test]
pub fn mutable_field_writes_allow_disjoint_and_final_shared_reads() {
    Case::new(
        r#"
d:@"debug"
value:={->left:=1;->right:=2}
left:&(value.left)
value.right=3
d.print(*left)
value.left=*left+1
whole:&value
value.right=whole.left+2
d.print(value.left);d.print(value.right)
nested:={->inner:={->left:=10;->right:=20};->other:=30}
view:&(nested.inner.left)
nested.inner.right=21
nested.other=31
d.print(*view)
nested.inner.left=*view+1
d.print(nested.inner.left);d.print(nested.inner.right);d.print(nested.other)
"#,
    )
    .runs(b"1\n2\n4\n10\n11\n21\n31\n");
}

#[test]
pub fn mutable_field_rhs_replacement_and_early_exits_preserve_store_order() {
    Case::new(
        r#"
d:@"debug"
value:={->left:=1;->right:=2}
value.left={d.print("rhs");value={->left:=3;->right:=4};->7}
d.print(value.left);d.print(value.right)
nested:={->inner:={->left:=1;->right:=2};->other:=3}
nested.inner.left={nested.inner={->left:=4;->right:=5};->9}
d.print(nested.inner.left);d.print(nested.inner.right);d.print(nested.other)
'out{nested.inner.left={d.print("leave");'out.leave();->99}}
d.print(nested.inner.left)
i:=0
'loop{i=i+1;value.left={|i<2|'loop.restart();->11}}
d.print(value.left)
"#,
    )
    .runs(b"rhs\n7\n4\n9\n5\n3\nleave\n9\n11\n");
    let source = "d:@\"debug\";value:={->n:=1};value.n={d.print(\"rhs\");d.panic(\"stop\")}";
    let start = source.find("d.panic(\"stop\")").unwrap();
    let end = start + "d.panic(\"stop\")".len();
    let case = Case::new(source);
    for profile in ["debug", "release"] {
        let output = case.command("run", &["--profile", profile]);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, b"rhs\n");
        assert_eq!(
            output.stderr,
            format!("panic[P006]: stop at bytes {start}..{end}\n").as_bytes()
        );
    }
}

#[test]
pub fn mutable_field_writes_preserve_sibling_facts_and_invalidate_target_facts() {
    Case::new(
        r#"
d:@"debug"
value:={->left<int32><string>:=1;->right<int32><string>:=2}
|value.left<int32>|{value.right="new";copy<int32>:value.left;d.print(copy)}
"#,
    )
    .runs(b"1\n");
    let source = "value:={->left<int32><string>:=1;->right<int32>:=2};|value.left<int32>|{value.left=\"new\";copy:value.left~<int32>}";
    let output = Case::new(source).command("check", &["--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E208\""));
}

#[test]
pub fn mutable_field_writes_reject_live_overlapping_loans() {
    for source in [
        "v:={->n:=1};r:&(v.n);v.n=2;x:*r",
        "v:={->a:=1;->b:=2};r:&v;v.b=3;x:r.a",
        "v:={->inner:={->n:=1}};r:&(v.inner);v.inner.n=2;x:r.n",
        "v:={->inner:={->n:=1}};r:&(v.inner.n);v.inner={->n:=2};x:*r",
        "v:={->a:=1;->b:=2};r:&(v.a);v.b={v={->a:=3;->b:=4};->5};x:*r",
        "v:={->n:=1};r:&(v.n);i:=0;'loop{x:*r;v.n=2;i=i+1;|i<2|'loop.restart()}",
    ] {
        let output = Case::new(source).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E302\""), "{source}: {error}");
    }
}

#[test]
pub fn mutable_fields_check_shapes_mutability_and_storage_boundaries() {
    Case::new("v:{->n:=1};v.n=2").runs(b"");
    Case::new("v:={->inner:{->n:=1}};v.inner.n=2").runs(b"");
    Case::new("<M>:<{n<int32>:=}>;f<null>:(v<M>){v.n=2}").runs(b"");
    Case::new("v:={->n:=1};v.{$.n=2}").runs(b"");
    for (source, code) in [
        (
            "x<uint8>:1;<A>:<{x<uint16>:=;y<uint8>}>;<B>:<{x<uint16>:=;y<uint16>}>;rows<A[1]><B[1]>:[{->x:=300;->y:x}]",
            "B001",
        ),
        ("v:={->n:1};v.n=2", "E305"),
        ("<M>:<{n<int32>:=}>;v<M>:{->n:1}", "E206"),
        ("<I>:<{n<int32>}>;v<I>:{->n:=1}", "E206"),
        ("f:(flag<boolean>){|flag|->n:=1;|!flag|->n:2}", "E206"),
        ("<I>:<{n<int32>}>;v:{->n:=1};copy<I>:v", "E207"),
        ("v:={->n:=1};v.n=\"wrong\"", "E207"),
        (
            "owner:1;n:=2;v:'out{'loop{'out->view:=&owner;view=&owner;n=n-1;|n>0|'loop.restart()}}",
            "B001",
        ),
        ("v:={->n:=1};r:&v;r.n=2", "B001"),
        ("v:={->n:=1};r:&v;(*r).n=2", "B001"),
        ("{->n:=1}.n=2", "B001"),
        ("v:={->n:=1};r:&!(v.n);v.n=2;w:*r", "E302"),
    ] {
        let case = Case::new(source);
        for profile in ["debug", "release"] {
            let output = case.command("build", &["--json", "--profile", profile]);
            assert_eq!(output.status.code(), Some(1), "{source}");
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(
                error.contains(&format!("\"code\":\"{code}\"")),
                "{source}: {error}"
            );
            assert!(!case.path.join("build").exists());
        }
    }
}
