use super::Case;

#[test]
pub fn element_borrows_keep_original_storage_and_end_at_final_use() {
    Case::new(
        r#"
d:@"debug"
values<int32[3]>:=[7,7]
copy:values
parent:&values
index:=1
first:&(values[1])
same:&(parent[index])
second:&(values[2])
other:&(copy[1])
position:=2
index_view:&position
selected:&(values[*index_view])
position=1
d.print(selected==second)
d.print(first==same)
d.print(first==second)
d.print(first==other)
d.print(&*same==same)
d.print(*second)
values=[30]
d.print(values[1])
text:{words:["inside"];view:&(words[1]);->*view}
d.print(text)
"#,
    )
    .runs(b"true\ntrue\nfalse\nfalse\ntrue\n7\n30\ninside\n");
}

#[test]
pub fn element_borrows_compose_nested_lists_records_and_reborrows() {
    Case::new(
        r#"
d:@"debug"
<Point>:<{x<int32>;y<int32>}>
<Row>:<{point<Point>;name<string>}>
rows<Row[3]>:=[{->point:{->x:11;->y:12};->name:"first"},{->point:{->x:21;->y:22};->name:"second"}]
row:&(rows[2])
x:&(rows[2].point.x)
same:&(row.point.x)
d.print(x==same)
d.print(*same)
name:&(row.name)
d.print(*name)
rows=[]
<Holder>:<{values<int32[3][2]>}>
holder<Holder>:={->values:[[3],[4,5]]}
view:&(holder.values[2][2])
d.print(*view)
holder={->values:[[6],[7]]}
d.print(holder.values[1][1])
"#,
    )
    .runs(b"true\n21\nsecond\n5\n6\n");
}

#[test]
pub fn element_borrow_functions_and_dispatch_preserve_input_lifetimes() {
    Case::new(
        r#"
d:@"debug"
head<&int32>:(items<&int32[3]>,position<usize>){->&(items[position])}
forward<&int32>:(items<&int32[3]>,position<usize>){->head(items,position)}
copy<null>:(items<int32[3]>){view:&(items[1]);d.print(*view)}
values<int32[3]>:=[10,20]
view:forward(&values,2)
d.print(view==&(values[2]))
d.print(*view)
copy(values)
values.{view:&($[1]);d.print(*view)}
shared:(&values).{->&($[1])}
d.print(*shared)
values=[30]
d.print(values[1])
optional<null>:(present<boolean>){
    owner<int32[3]>:=[10]
    parent<&int32[3]><null>:{|present|->&owner}
    |parent<&int32[3]>|{element:&(parent[1]);d.print(*element)}
    |parent<null>|owner=[99]
    owner=[30]
    d.print(owner[1])
}
optional(true)
optional(false)
"#,
    )
    .runs(b"true\n20\n10\n10\n10\n30\n10\n30\n30\n");
}

#[test]
pub fn element_borrow_parents_indices_and_early_exits_evaluate_once() {
    Case::new(
        r#"
d:@"debug"
parent<&int32[3]>:(items<&int32[3]>){d.print("parent");->items}
position<usize>:(){d.print("index");->1}
values<int32[3]>:=[9]
d.print(&(parent(&values)[position()])==&(values[1]))
'out {unused:&(parent(&values)[{d.print("leave");'out.leave();->1}]);d.print("unreachable")}
empty<int32[0]>:[]
'out {unused:&(empty[{'out.leave()}]);d.print("unreachable")}
'out {unused:&(values[{values=[];'out.leave();->1}])}
d.print(values.size())
|false|{unused:&(values[0])}
skipped:true||{unused:&(values[0]);->true}
d.print(skipped)
"#,
    )
    .runs(b"parent\nindex\ntrue\nparent\nleave\n0\ntrue\n");
}

#[test]
pub fn element_borrows_enforce_bounds_lifetimes_and_remaining_boundaries() {
    for (source, code) in [
        ("values:[1];view:&(values[0])", "E101"),
        ("values:[1];view:&(values[-1])", "E101"),
        ("values<int32[3]>:[1];view:&(values[2])", "E101"),
        ("values<int32[0]>:[];view:&(values[1])", "E101"),
        ("view:{values:[1];->&(values[1])}", "E303"),
        ("bad<&int32>:(values<int32[2]>){->&(values[1])}", "E303"),
        ("values:[1];view:values.{->&($[1])}", "E303"),
        ("view:&([1,2][1]);copy:*view", "E303"),
        (
            "make<int32[2]>:(){->[1]};view:&(make()[1]);copy:*view",
            "E303",
        ),
        ("values:=[1];view:&!(values[1]);values[1]=2;v:*view", "E302"),
        ("owner:1;values:[&owner];view:&(values[1])", "B001"),
        ("values:[{->1;->name:2}];view:&(values[1]~<int32>)", "E208"),
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

#[test]
pub fn element_borrows_protect_parent_storage_and_ignored_input_bounds() {
    for source in [
        "values<int32[2]>:=[1];view:&(values[1]);values=[];copy:*view",
        "values<int32[2]>:=[1];unused:&(values[{values=[];->1}])",
        "values<int32[2]>:=[1];same:&(values[1])=={values=[];->&(values[1])}",
        "head<&int32>:(values<&int32[2]>,other<&string>){->&(values[1])};values<int32[2]>:=[1];other:=\"before\";view:head(&values,&other);other=\"after\";copy:*view",
        "pick<&int32[2]>:(values<&int32[2]>,other<&string>){->values};values<int32[2]>:=[1];other:=\"before\";unused:&(pick(&values,&other)[{other=\"after\";->1}])",
        "head<&int32>:(values<&int32[2]>,index<&int32>){->&(values[*index])};values<int32[2]>:=[1];index:=1;view:head(&values,&index);index=2;copy:*view",
        "values<int32[2]>:=[1];holder:{->view:&(values[1])};values=[];copy:holder.view",
        "values<int32[2]>:=[1];flag:=true;|flag|{view:&(values[1]);values=[];copy:*view}",
    ] {
        let output = Case::new(source).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E302\""), "{source}: {error}");
    }
    let source = "head<&int32>:(values<&int32[2]>,other<&string>){->&(values[1])};values<int32[2]>:[1];view:{other:\"local\";->head(&values,&other)}";
    let output = Case::new(source).command("check", &["--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E303\""));
    let source = "head<&int32>:(values<&int32[2]>){->&(values[1])};values:[1,2];bad:{head(&values);local:3;->&local}";
    let output = Case::new(source).command("check", &["--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E303\""));
}

#[test]
pub fn element_borrow_dynamic_bounds_report_original_index_and_length() {
    for (capacity, initial, ty, index, shown, length) in [
        (3, "[10]", "int8", "-1", "-1", 1),
        (3, "[10]", "usize", "2", "2", 1),
        (
            3,
            "[10]",
            "uint64",
            "18446744073709551615",
            "18446744073709551615",
            1,
        ),
        (0, "[]", "int64", "1", "1", 0),
    ] {
        let access = "&(items[{d.print(\"index\");->position}])";
        let source = format!(
            "#é🙂#\nd:@\"debug\";get<&int32>:(items<&int32[{capacity}]>,position<{ty}>){{->{access}}};values<int32[{capacity}]>:{initial};view:get(&values,{index});d.print(*view)"
        );
        let start = source.find(access).unwrap();
        let end = start + access.len();
        let case = Case::new(&source);
        for profile in ["debug", "release"] {
            let output = case.command("run", &["--profile", profile]);
            assert_eq!(output.status.code(), Some(1), "{source}");
            assert_eq!(output.stdout, b"index\n");
            assert_eq!(output.stderr, format!("panic[P001]: index {shown} is outside initialized length {length} at bytes {start}..{end}\n").as_bytes());
        }
    }
}
