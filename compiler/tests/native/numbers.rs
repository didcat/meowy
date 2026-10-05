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
1.0:2.5
d.print(1.0)
128<int8>:7
d.print(-128)
f<int32>:(2<int32>){->2+3}
d.print(f(4))
"#,
    )
    .runs(b"4\n3\n2\n1\n1\n7\n2.5\n-7\n7\n");
}
