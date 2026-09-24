use super::Case;

#[test]
pub fn intrinsics_alias_and_names_shadow_normally() {
    Case::new(
        r#"
core:@"core"
d:@"debug"
show:d.print
{
    true:"shadowed"
    show(true)
    show(core.true)
    debug:{->print:9}
    show(debug.print)
}
'done {
    finish:'done.leave
    show("done")
    finish()
    show("unreachable")
}
"#,
    )
    .runs(b"shadowed\ntrue\n9\ndone\n");
}

#[test]
pub fn booleans_short_circuit_and_floats_preserve_types() {
    Case::new(
        r#"
d:@"debug"
probe<boolean>:(){d.print("probe");->true}
d.print(false&&probe())
d.print(true||probe())
d.print(1.5+2.25)
d.print("hé\0".size())
"#,
    )
    .runs(b"false\ntrue\n3.75\n4\n");
}

#[test]
pub fn integer_boundaries_and_bitwise_values_are_exact() {
    Case::new(
        r#"
d:@"debug"
a<int8>:-128
b<uint64>:18446744073709551615
c<int64>:-9223372036854775808
d.print(a)
d.print(b)
d.print(c)
d.print((5&3)^8|2)
"#,
    )
    .runs(b"-128\n18446744073709551615\n-9223372036854775808\n11\n");
}

#[test]
pub fn unary_results_enter_expected_unions_after_operand_evaluation() {
    Case::new(
        r#"
d:@"debug"
<Logic>:<boolean><null>
logic<Logic[1]>:[!true]
choice<int32[1]><Logic[1]>:[!true]
first:logic[1]
|first<boolean>|d.print(first~<boolean>)
|choice<Logic[1]>|{value:choice[1];|value<boolean>|d.print(value~<boolean>)}
<Number>:<int8><null>
number<int8>:5
single<Number[2]>:[-number,~number]
selected<Number[2]><string[2]>:[-number,~number]
negative:single[1]
|negative<int8>|d.print(negative~<int8>)
|selected<Number[2]>|{value:selected[2];|value<int8>|d.print(value~<int8>)}
<Float>:<float32><null>
value<Float>:-({->1.5})
|value<float32>|d.print(value~<float32>)
"#,
    )
    .runs(b"false\nfalse\n-5\n-6\n-1.5\n");
}

#[test]
pub fn foundation_declarations_preserve_aliases_and_ordinary_shadowing() {
    let source = r#"m:@"memory";s:@"strings";heap:m.heap;copy:s.copy;kind:s.Owned;<T>:<(kind)>;d:@"debug";d.print("ready");{m:{->heap:7};d.print(m.heap)};strings:@"debug";strings.print("alias")"#;
    let case = Case::new(source);
    for profile in ["debug", "release"] {
        let output = case.command("run", &["--profile", profile]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"ready\n7\nalias\n");
        assert!(output.stderr.is_empty());
    }
}

#[test]
pub fn heap_handles_copy_through_storage_calls_records_lists_and_unions() {
    let source = r#"
m:@"memory"
d:@"debug"
pass<m.Allocator>:(value<m.Allocator>){->value}
cell<&m.Allocator>:(value<&m.Allocator>){->value}
maybe<m.Allocator><null>:(keep<boolean>){|keep|->m.heap}
a<m.Allocator>:m.heap
b:a
|a<m.Allocator>|d.print(true)
d.print(&a==&b)
d.print(&a==&a)
d.print(cell(&a)==&a)
items:=[a,b]
i:=1
items[i]=m.heap
d.print(items.size())
view:&(items[i])
copy:pass(*view)
record:{->handle:copy}
|record.handle<m.Allocator>|d.print(true)
present:maybe(true)
|present<m.Allocator>|d.print(true)
absent:maybe(false)
|absent<null>|d.print(true)
"#;
    let case = Case::new(source);
    for profile in ["debug", "release"] {
        let output = case.command("run", &["--profile", profile]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.stdout,
            b"true\nfalse\ntrue\ntrue\n2\ntrue\ntrue\ntrue\n"
        );
        assert!(output.stderr.is_empty());
    }
}
