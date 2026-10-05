use super::Case;

#[test]
pub(crate) fn numeric_names_resolve_lexically_in_both_profiles() {
    Case::new(
        r#"
d:@"debug"
1:2
d.print(1+1)
{1:3;d.print(1)}
d.print(1)
d.print(01)
d.print(0x1)
0x1:7
d.print(0x1)
1e0:2.5
d.print(1e0)
128<int8>:7
d.print(-128)
f<int32>:(2<int32>){->2+3}
d.print(f(4))
"#,
    )
    .runs(b"4\n3\n2\n1\n1\n7\n2.5\n-7\n7\n");
}

#[test]
pub(crate) fn numeric_storage_keeps_mutation_borrows_and_narrowing() {
    Case::new(
        r#"
d:@"debug"
1:=7
r:&1
d.print(*r)
1=8
q:&!1
*q=9
d.print(1)
2:({->1:=10})
(2).1=11
d.print((2).1)
3<int32[02]>:=[12,13]
3[01]=14
d.print(3[01])
f<int32>:(2<int32>)'done{|2==0|{'done->15;'done.leave()};->f(2-01)}
d.print(f(4))
"#,
    )
    .runs(b"7\n9\n11\n14\n15\n");
}

#[test]
pub(crate) fn core_literal_bypasses_shadowing_with_contextual_numeric_types() {
    Case::new(
        r#"
d:@"debug"
c:@"core"
lit:c.literal
1:lit(2)
0x1:true
128:7
d.print(1+1)
d.print(lit(1))
d.print(c.literal(0x1))
x<int8>:lit(-128)
y<uint64>:lit(18446744073709551615)
f<float32>:lit(1.5)
d.print(x)
d.print(y)
d.print(f)
"#,
    )
    .runs(b"4\n1\n1\n-128\n18446744073709551615\n1.5\n");
}

#[test]
pub(crate) fn numeric_roots_select_members_and_unbound_roots_remain_literals() {
    Case::new(
        r#"
d:@"debug"
10:{->4:{->"4"};->name:7;->8:{->9:12}}
d.print(10.4)
d.print(10.name)
d.print(10.8.9)
d.print(11.4)
d.print(11)
d.print(@"core".literal(10.4))
{10:{->4:13};d.print(10.4)}
d.print(10.4)
"#,
    )
    .runs(b"4\n7\n12\n11.4\n11\n10.4\n13\n4\n");
}

#[test]
pub(crate) fn numeric_member_failures_never_retry_decimal_construction() {
    for (source, code) in [
        ("10:{->4:7};x:10.5", "E201"),
        ("10:7;x:10.4", "E201"),
        ("10:{4:7};x:10.4", "E201"),
        ("10.4:7", "E004"),
        ("10.4:=7", "E004"),
        ("10:{->4:7};10.4:8", "E004"),
    ] {
        let result = Case::new(source).command("check", &["--json"]);
        assert_eq!(result.status.code(), Some(1), "{source}");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(code),
            "{source}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[test]
pub(crate) fn numeric_member_places_keep_writes_and_borrows_in_both_profiles() {
    Case::new(
        r#"
d:@"debug"
10:{->4:=7;->8:{->9:=8}}
10.4=11
{r:&!(10.8.9);*r=12}
d.print(10.4)
d.print(10.8.9)
r:&(10.4)
d.print(*r)
10.4=13
d.print(10.4)
20:{->2<int32[2]>:=[4,5]}
20.2[1]=9
d.print(20.2[1])
d.print(20.2[2])
"#,
    )
    .runs(b"11\n12\n11\n13\n9\n5\n");
}
