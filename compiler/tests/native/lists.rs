use super::Case;

#[test]
pub fn bounded_lists_preserve_element_types_capacity_and_value_copies() {
    Case::new(
        r#"
d:@"debug"
byte<uint8>:7
values:[byte,8]
accept<uint8>:(value<uint8>){->value}
d.print(accept(values[2]))
capacity:2+2
original<int32[capacity]>:[10]
changed:original.add(20)
d.print(original.size())
d.print(changed.size())
d.print(changed[2])
copy<int32[4]>:=changed
copy=[]
d.print(copy.size())
d.print(changed[1])
empty<null[0]>:[]
d.print(empty.size())
growing<int32[3]>:=[]
next:=1
'fill {
    growing=growing.add(next)
    next=next+1
    |next<=3|'fill.restart()
}
d.print(growing.size())
d.print(growing[3])
"#,
    )
    .runs(b"8\n1\n2\n20\n0\n10\n0\n3\n3\n");
}

#[test]
pub fn bounded_lists_keep_records_unions_and_nested_values() {
    Case::new(
        r#"
d:@"debug"
<Row>:<{name<string>;value<int32>}>
rows<Row[3]>:[{->name:"first";->value:11},{->name:"second";->value:22}]
d.print(rows[2].name)
d.print(rows[1].value)
<Value>:<int32><string>
member<Value>:3
mixed:[member,"text"]
first:mixed[1]
second:mixed[2]
|first<int32>|d.print(first<int32>)
|second<string>|d.print(second<string>)
nested<int32[3][2]>:[[1],[2,3]]
d.print(nested[2][2])
d.print(nested[1].size())
<Numbers>:<int32[2]>
<Words>:<string[2]>
numbers<Numbers>:[5]
choice<Numbers><Words>:numbers
|choice<Numbers>|d.print(choice<Numbers>[1])
d.print(choice==choice)
"#,
    )
    .runs(b"second\n11\n3\ntext\n3\n1\n5\ntrue\n");
}

#[test]
pub fn list_equality_uses_initialized_elements_and_element_semantics() {
    Case::new(
        r#"
d:@"debug"
a<int32[4]>:=[1,2,3]
a=[]
b<int32[4]>:[]
d.print(a==b)
d.print(a!=b.add(1))
x<float64[3]>:[-0.0]
y<float64[3]>:[0.0]
d.print(x==y)
nan:0.0/0.0
n:[nan]
d.print(n==n)
s<string[3]>:["same"]
t<string[3]>:["same"]
d.print(s==t)
left<int32[3][2]>:[[1],[2,3]]
right<int32[3][2]>:[[1],[2,3]]
d.print(left==right)
<Row>:<{value<int32>;name<string>}>
r<Row[2]>:[{->value:1;->name:"one"}]
q<Row[2]>:[{->value:1;->name:"one"}]
d.print(r==q)
<Value>:<int32><string>
u<Value[2]>:[1,"two"]
v<Value[2]>:[1,"two"]
d.print(u==v)
"#,
    )
    .runs(b"true\ntrue\ntrue\nfalse\ntrue\ntrue\ntrue\ntrue\n");
}

#[test]
pub fn list_receivers_are_copied_before_argument_effects() {
    Case::new(
        r#"
d:@"debug"
values<int32[3]>:=[1,2]
item:values[{values=[];->1}]
d.print(item)
d.print(values.size())
values=[1]
result:values.add({values=[2,3];->4})
d.print(result[1])
d.print(result[2])
d.print(values[1])
d.print(result.size())
make<int32[3]>:(){d.print("receiver");->[9]}
index<usize>:(){d.print("index");->1}
d.print(make()[index()])
'out {unused:values[{'out.leave();->1}];d.print("unreachable")}
'out {unused:values.add({'out.leave();->1});d.print("unreachable")}
d.print("done")
"#,
    )
    .runs(b"1\n0\n1\n4\n2\n2\nreceiver\nindex\n9\ndone\n");
}

#[test]
pub fn lists_reject_static_bounds_and_incompatible_elements() {
    for (source, code) in [
        ("values:[1,2];item:values[0]", "E101"),
        ("values<int32[4]>:[1];item:values[2]", "E101"),
        ("values<int32[0]>:[];item:values[1]", "E101"),
        ("values<int32[1]>:[1,2]", "E103"),
        ("values:[1];other:values.add(2)", "E103"),
        ("values<int32[0]>:[];other:values.add(1)", "E103"),
        ("values<int32[-1]>:[]", "E104"),
        (
            "f<null>:(capacity<usize>){values<int32[capacity]>:[]}",
            "E104",
        ),
        ("capacity<uint8>:255;values<int32[capacity+1]>:[]", "E107"),
        ("a<uint8>:1;b<uint16>:2;values<int32[a+b]>:[]", "E213"),
        ("values:[1,\"two\"]", "E207"),
        ("a<uint8>:1;b<uint16>:2;values:[a,b]", "E207"),
        ("a:{->1;->field:2};values:[a,3]", "E207"),
        ("values<int32[2]>:[1,\"two\"]", "E207"),
        ("a:[1];b<int32[2]>:a", "E207"),
        ("values:[1];size:values.size(1)", "E212"),
    ] {
        let case = Case::new(source);
        for profile in ["debug", "release"] {
            let result = case.command("build", &["--json", "--profile", profile]);
            assert_eq!(result.status.code(), Some(1), "{source}");
            let error = String::from_utf8_lossy(&result.stderr);
            assert!(
                error.contains(&format!("\"code\":\"{code}\"")),
                "{source}: {error}"
            );
            assert!(!case.path.join("build").exists());
        }
    }
}

#[test]
pub fn list_dynamic_failures_follow_argument_effects_in_both_profiles() {
    for (source, code) in [
        (
            "d:@\"debug\";read<int32>:(values<int32[3]>,position<usize>){->values[{d.print(\"argument\");->position}]};read([1],2)",
            "P001",
        ),
        (
            "d:@\"debug\";read<int32>:(values<int32[3]>,position<int32>){->values[{d.print(\"argument\");->position}]};read([1],-1)",
            "P001",
        ),
        (
            "d:@\"debug\";read<int32>:(values<int32[3]>,position<uint64>){->values[{d.print(\"argument\");->position}]};read([1],18446744073709551615)",
            "P001",
        ),
        (
            "d:@\"debug\";append<int32[1]>:(values<int32[1]>){->values.add({d.print(\"argument\");->2})};append([1])",
            "P003",
        ),
    ] {
        let case = Case::new(source);
        for profile in ["debug", "release"] {
            let result = case.command("run", &["--profile", profile]);
            assert_eq!(result.status.code(), Some(1), "{source}");
            assert_eq!(result.stdout, b"argument\n", "{source}");
            let error = String::from_utf8_lossy(&result.stderr);
            assert!(error.contains(code), "{source}: {error}");
        }
    }
}

#[test]
pub fn whole_list_borrows_keep_owner_identity_and_end_at_final_use() {
    Case::new(
        r#"
d:@"debug"
values<int32[3]>:=[10]
view:&values
same<&int32[3]>:(value<&int32[3]>){->value}
alias:same(view)
d.print(alias==view)
d.print((*alias)[1])
d.print(view[1])
d.print(view.size())
values=[20,30]
d.print(values.size())
<Row>:<{values<int32[3]>}>
row<Row>:={->values:[4]}
field:&(row.values)
d.print((*field)[1])
row={->values:[5,6]}
d.print(row.values[2])
owner:=7
borrow:&owner
items:[*borrow,{owner=8;->9}]
d.print(items[1])
d.print(owner)
last:&values
d.print((*last)[{values=[];->1}])
d.print(values.size())
"#,
    )
    .runs(b"true\n10\n10\n1\n2\n4\n6\n7\n8\n20\n0\n");
}

#[test]
pub fn list_borrow_conflicts_and_unavailable_operations_stay_explicit() {
    for (source, code) in [
        (
            "values<int32[2]>:=[1];view:&values;values=[];copy:*view",
            "E302",
        ),
        ("owner:=1;view:&owner;values:[{owner=2;->3},*view]", "E302"),
        ("view:{values:[1];->&values}", "E303"),
        (
            "keep<&int32[2]>:(first<&int32[2]>,other<&int32>){->first};values<int32[2]>:=[1];other:=2;view:keep(&values,&other);other=3;copy:*view",
            "E302",
        ),
        ("values<int32[]>:[]", "B001"),
        ("values<uint8[65537]>:[]", "B001"),
        ("values:[\"name\":1]", "B001"),
        ("owner:1;values:[&owner]", "B001"),
        ("values:[1];slice:values.slice()", "B001"),
        ("values:[1];removed:values.remove(1)", "B001"),
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
pub fn list_length_proofs_follow_completing_paths() {
    Case::new(
        r#"
d:@"debug"
flag:=true
values<int32[2]>:'choose {
    |flag|{'choose->[10,20];'choose.leave()}
    ->[30]
}
d.print(values[2])
short<int32[2]>:'choose {
    |flag|{'choose->[40];'choose.leave()}
    ->[50,60]
}
longer:short.add(70)
d.print(longer[2])
"#,
    )
    .runs(b"20\n70\n");
    let source = "values<int32[3]>:'out{->[];'out.leave();->[1,2]};item:values[1]";
    let result = Case::new(source).command("check", &["--json"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("\"code\":\"E101\""));
}

#[test]
pub(crate) fn ordinary_extent_roots_preserve_capacity_and_required_inputs() {
    Case::new(
        "d:@\"debug\";n<uint8>:2;v<int32[n+1]>:[7,9];copy<int32[((n)+1)]>:v;d.print(copy[2]);<T>:{m:{->2};-><int32[m]>};other<T>:[3,4];d.print(other[2]);empty<int32[0]>:[];d.print(empty.size())",
    ).runs(b"9\n4\n0\n");
    for (source, code) in [
        ("n<uint8>:255;|false|v<int32[n+1]>:[]", "E107"),
        ("n:=2;f<null>:(){v<int32[n]>:[]}", "E104"),
        ("v<int32[({->2})]>:[]", "B001"),
    ] {
        let case = Case::new(source);
        for profile in ["debug", "release"] {
            let output = case.command("check", &["--profile", profile, "--json"]);
            assert_eq!(output.status.code(), Some(1));
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains(&format!("\"code\":\"{code}\"")), "{error}");
        }
    }
}

#[test]
pub(crate) fn ordinary_extent_roots_keep_original_imported_errors() {
    let source = "limit<uint8>:255;-><Items>:<int32[limit+1]>";
    let case = super::file_modules::case(
        "m:@\"./facade.mwy\"",
        &[
            ("data.mwy", source),
            ("facade.mwy", "m:@\"./data.mwy\";-><Items>:<m.Items>"),
        ],
    );
    for action in ["check", "build", "run"] {
        let output = case.command(action, &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E107\""), "{error}");
        assert!(
            error.contains(&format!(
                "\"path\":\"{}\"",
                case.path.join("data.mwy").display()
            )),
            "{error}"
        );
        assert!(
            error.contains(&format!("\"start\":{}", source.find("limit+1").unwrap())),
            "{error}"
        );
    }
}

#[test]
pub(crate) fn ordinary_constructor_roots_preserve_aliases_exports_and_computed_operands() {
    super::file_modules::case(
        "m:@\"./facade.mwy\";d:@\"debug\";copy<m.Items>:m.items;d.print(copy[2]);<C>:{-><uint8[({->1+1})]>};other<C>:[3,4];d.print(other[2])",
        &[
            ("data.mwy", "d:@\"debug\";d.print(1);-><Items>:<uint8[1+1]>;->items<Items>:[7,9]"),
            ("facade.mwy", "m:@\"./data.mwy\";d:@\"debug\";d.print(2);-><Items>:<m.Items>;->items<m.Items>:m.items"),
        ],
    ).runs(b"1\n2\n9\n4\n");
    for source in [
        "<T>:<{a<({-><int32>})>;b<int32[({->2})]>}>",
        "v<int32[({->2})]>:[]",
        "->v<int32[({->2})]>:[]",
    ] {
        let case = Case::new(source);
        for profile in ["debug", "release"] {
            let output = case.command("check", &["--profile", profile, "--json"]);
            assert_eq!(output.status.code(), Some(1));
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains("\"code\":\"B001\""), "{error}");
        }
    }
}

#[test]
pub(crate) fn ordinary_constructor_roots_preserve_source_errors_across_facades() {
    for source in ["-><Items>:<int32[1/0]>", "->items<int32[1/0]>:[]"] {
        let case = super::file_modules::case(
            "m:@\"./facade.mwy\"",
            &[
                ("data.mwy", source),
                ("facade.mwy", "m:@\"./data.mwy\";->tag:1"),
            ],
        );
        for action in ["check", "build", "run"] {
            let output = case.command(action, &["--json"]);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains("\"code\":\"E107\""), "{error}");
            assert!(
                error.contains(&format!(
                    "\"path\":\"{}\"",
                    case.path.join("data.mwy").display()
                )),
                "{error}"
            );
            assert!(
                error.contains(&format!("\"start\":{}", source.find("1/0").unwrap())),
                "{error}"
            );
        }
    }
}

#[test]
pub(crate) fn signature_roots_preserve_parameter_types_through_bodies_and_facades() {
    super::file_modules::case(
        "m:@\"./facade.mwy\";d:@\"debug\";d.print(m.first(9,[7,8]))",
        &[
            ("data.mwy", "width:2;d:@\"debug\";d.print(1);->first<int32>:(width<int32>,items<int32[width]>){->items[1]}"),
            ("facade.mwy", "m:@\"./data.mwy\";d:@\"debug\";d.print(2);->first<(int32,int32[2])->int32>:m.first"),
        ],
    ).runs(b"1\n2\n7\n");
    Case::new(
        "d:@\"debug\";width:2;f<(int32,int32[2])->int32>;f<int32>:(width<int32>,items<int32[width]>){->items[2]};d.print(f(9,[7,8]))",
    ).runs(b"8\n");
}

#[test]
pub(crate) fn signature_roots_keep_forward_and_export_source_errors() {
    for (source, facade, path) in [
        (
            "f<(int32[2])->int32[2]>;f<int32[1/0]>:(n<int32[2/0]>){->n}",
            "m:@\"./data.mwy\"",
            "data.mwy",
        ),
        (
            "->f<int32>:(n<int32>){->n}",
            "m:@\"./data.mwy\";->f<(int32[1/0])->int32>:m.f",
            "facade.mwy",
        ),
    ] {
        let case = super::file_modules::case(
            "m:@\"./facade.mwy\"",
            &[("data.mwy", source), ("facade.mwy", facade)],
        );
        let text = if path == "data.mwy" { source } else { facade };
        for action in ["check", "build", "run"] {
            let output = case.command(action, &["--json"]);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains("\"code\":\"E107\""), "{error}");
            assert!(
                error.contains(&format!("\"path\":\"{}\"", case.path.join(path).display())),
                "{error}"
            );
            assert!(
                error.contains(&format!("\"start\":{}", text.find("1/0").unwrap())),
                "{error}"
            );
        }
    }
}
