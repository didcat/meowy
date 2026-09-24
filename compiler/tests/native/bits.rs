use super::Case;

#[test]
pub(crate) fn bit_functions_keep_exact_widths_and_source_order_in_both_profiles() {
    let case = Case::new(
        r#"
b:@"bits"
d:@"debug"
mark<int32>:(n<int32>){d.print(n);->n}
combine:b.and
flip:b.not
small<uint8>:flip(0)
signed<int8>:flip(-128)
wide<uint64>:b.xor(18446744073709551615,0)
d.print(small)
d.print(signed)
d.print(wide)
d.print(b.or(b.xor(combine(5,3),8),2))
d.print(mark(1).(b.or,mark(2)))
d.print(b.and(mark(0),mark(3)))
"#,
    );
    for profile in ["debug", "release"] {
        let output = case.command("run", &["--profile", profile]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.stdout,
            b"255\n127\n18446744073709551615\n11\n1\n2\n3\n0\n3\n0\n"
        );
        assert!(output.stderr.is_empty());
    }
}

#[test]
pub(crate) fn bitwise_operator_spellings_are_rejected_but_other_punctuation_remains() {
    for source in ["x:1&2", "x:1|2", "x:1^2", "x:~1"] {
        let result = Case::new(source).command("check", &["--json"]);
        assert_eq!(result.status.code(), Some(1), "{source}");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("E004"),
            "{source}"
        );
    }
    Case::new(
        r#"
d:@"debug"
x:=7
r:&x
d.print(*r)
d.print(true&&(!false||false))
|x<int32>|d.print(x~<int32>)
d.print(x%3)
"#,
    )
    .runs(b"7\ntrue\n7\n1\n");
}
