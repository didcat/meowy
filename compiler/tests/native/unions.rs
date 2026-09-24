use super::Case;

#[test]
pub fn conditional_fields_and_primaries_have_real_null_defaults() {
    Case::new(
        r#"
d:@"debug"
make:(flag<boolean>){|flag|->name:"hello"}
a<{name<string><null>}>:make(false)
b:make(true)
d.print(a.name)
d.print(b.name)
maybe<int8><null>:(flag<boolean>){|flag|->-128}
d.print(maybe(false))
d.print(maybe(true))
empty<{name<string><null>}>:{}
d.print(empty.name)
"#,
    )
    .runs(b"null\nhello\nnull\n-128\nnull\n");
}

#[test]
pub fn union_normalization_and_retagging_preserve_payloads() {
    Case::new(
        r#"
d:@"debug"
<Small>:<string><null><never><string>
<Wide>:<string><boolean><null>
widen<Wide>:(value<Small>){->value}
narrow<Small>:(value<Wide>) 'result {
    |value<boolean>|{'result->"boolean";'result.leave()}
    ->value
}
d.print(widen("payload"))
d.print(widen(null))
d.print(narrow(widen("same")))
d.print(narrow(true))
d.print(narrow(null))
"#,
    )
    .runs(b"payload\nnull\nsame\nboolean\nnull\n");
}

#[test]
pub fn union_equality_compares_only_the_active_variant() {
    Case::new(
        r#"
d:@"debug"
same<boolean>:(a<int32><string><null>,b<null><string><int32>){->a==b}
d.print(same(7,7))
d.print(same(7,"7"))
d.print(same("same","same"))
d.print(same("same","different"))
d.print(same(null,null))
d.print(same(null,""))
a<{name<string><null>}>:{}
b<{name<string><null>}>:{->name:""}
d.print(a==a)
d.print(a!=b)
"#,
    )
    .runs(b"true\nfalse\ntrue\nfalse\ntrue\nfalse\ntrue\ntrue\n");
}

#[test]
pub fn type_tests_narrow_values_fields_and_short_circuit_operands() {
    Case::new(
        r#"
d:@"debug"
inspect<null>:(value<int32><string><null>){
    |value<int32>|d.print(value+1)
    |value<string>&&value.size()>0|d.print(value~<string>)
    |value<null>|d.print("missing")
}
inspect(4)
inspect("text")
inspect(null)
fallback<string>:(value<string><null>) 'result {
    |value<null>|{'result->"fallback";'result.leave()}
    ->value~<string>
}
d.print(fallback(null))
d.print(fallback("kept"))
field<null>:(record<{name<string><null>}>){
    |record.name<string>|d.print(record.name.size())
    |record.name<null>||record.name.size()==0|d.print("empty")
}
field({->name:"four"})
field({})
"#,
    )
    .runs(b"5\ntext\nmissing\nfallback\nkept\n4\nempty\n");
}

#[test]
pub fn complementary_conditions_prove_exactly_one_emission() {
    Case::new(
        r#"
d:@"debug"
select<int32>:(value<string><null>){
    |value<null>|->1
    |!(value<null>)|->2
}
boolean<int32>:(flag<boolean>){
    |flag|->3
    |!flag|->4
}
d.print(select(null))
d.print(select("yes"))
d.print(boolean(true))
d.print(boolean(false))
"#,
    )
    .runs(b"1\n2\n3\n4\n");
}

#[test]
pub fn record_variants_keep_full_identity_and_nested_null_defaults() {
    Case::new(
        r#"
d:@"debug"
<Reading>:<{name<string><null>;count<int32>}>
read<null>:(value<Reading><string><null>){
    |value<Reading>|{
        d.print(value.count)
        d.print(value.name)
    }
    |value<null>|d.print("missing")
    |value<string>|d.print(value)
}
r<Reading>:{->count:7}
read(r)
read(null)
read("text")
|r<null>|d.print("incorrect")
wrapped<{reading<Reading><null>}>:{->reading:r}
|wrapped.reading<Reading>|d.print(wrapped.reading.name)
"#,
    )
    .runs(b"7\nnull\nmissing\ntext\nnull\n");
}

#[test]
pub fn predicate_operands_evaluate_once_even_for_known_results() {
    Case::new(
        r#"
d:@"debug"
make<int32><null>:(){d.print("union");->7}
scalar<int32>:(){d.print("scalar");->4}
|make()<int32>|d.print("matched")
|scalar()<null>|d.print("incorrect")
"#,
    )
    .runs(b"union\nmatched\nscalar\n");
}

#[test]
pub fn nullable_slot_defaults_reset_on_restart() {
    Case::new(
        r#"
d:@"debug"
make<{name<string><null>}>:(){
    first:=true
    ->'again {
        |first|{
            'again->name:"discarded"
            first=false
            'again.restart()
        }
    }
}
d.print(make().name)
"#,
    )
    .runs(b"null\n");
}

#[test]
pub fn expected_record_composition_preserves_fields_without_inventing_them() {
    Case::new(
        r#"
d:@"debug"
record<{name<string><null>;count<int32>}>:{
    ->{->name:"kept"}
    ->count:7
}
d.print(record.name)
d.print(record.count)
"#,
    )
    .runs(b"kept\n7\n");
}

#[test]
pub fn variant_fields_with_the_same_name_have_distinct_type_proofs() {
    Case::new(
        r#"
d:@"debug"
<Text>:<{name<string><null>}>
<Number>:<{name<int32><null>}>
read<null>:(value<Text><Number>){
    |value<Text>|{
        |value.name<string>|d.print(value.name.size())
        |value.name<null>|d.print("missing text")
    }
    |value<Number>|{
        |value.name<int32>|d.print(value.name+1)
        |value.name<null>|d.print("missing number")
    }
}
text<Text>:{->name:"test"}
number<Number>:{->name:8}
empty<Text>:{}
read(text)
read(number)
read(empty)
"#,
    )
    .runs(b"4\n9\nmissing text\n");
}

#[test]
pub fn branch_proofs_do_not_survive_mutation_or_continuing_arms() {
    for (source, code) in [
        (
            "f<null>:(x<string><null>){|x<null>|{};y:x~<string>}",
            "E208",
        ),
        (
            "x<string><null>:=\"ok\";|x<string>|{x=null;y:x~<string>}",
            "E208",
        ),
        (
            "x<{name<string><null>}>:={->name:\"ok\"};|x.name<string>|{x={};y:x.name~<string>}",
            "E208",
        ),
        ("f<int32>:(flag<boolean>){|flag|->1}", "E204"),
        ("f<int32><null>:(flag<boolean>){|flag|->1;->2}", "E205"),
        ("f<int32>:(x<int32>){|x>0|->1;|x>10|->2}", "E205"),
        ("x<int8><null>:128", "E216"),
        ("x<int8>:1;y<int16><null>:x", "E207"),
    ] {
        let result = Case::new(source).command("check", &["--json"]);
        assert_eq!(result.status.code(), Some(1), "{source}");
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr.contains(&format!("\"code\":\"{code}\"")),
            "{source}: {stderr}"
        );
    }
}

#[test]
pub fn independent_matchers_do_not_enumerate_every_path() {
    let params: Vec<_> = (0..24).map(|i| format!("a{i}<boolean>")).collect();
    let arms: String = (0..24).map(|i| format!("|a{i}|{{}};")).collect();
    let args = vec!["true"; 24].join(",");
    let source = format!(
        "d:@\"debug\";f<int32>:({}){{{arms}->7}};d.print(f({args}))",
        params.join(",")
    );
    Case::new(&source).runs(b"7\n");
}

#[test]
pub fn union_record_constructors_preserve_widths_defaults_and_discarded_fields() {
    Case::new(
        r#"
d:@"debug"
<R>:<{view<&int32><null>;count<int64>}>
owner:=42
present<R><null>:{->view:&owner;->count:7}
|present<R>|{|present.view<&int32>|d.print(*(present.view~<&int32>));d.print(present.count)}
owner=43
absent<R><null>:{->count:9}
empty_view<&int32><null>:null
|absent<R>|{d.print(absent.view==empty_view);d.print(absent.count)}
<P>:<{-><int64>;tag<string>}>
primary<P><null>:{->7;->tag:"ok"}
|primary<P>|{number<int64>:primary;d.print(number);d.print(primary.tag)}
<B>:<{value<uint8>;view<&int32>}>
small<B><null>:{->value:255;->view:&owner}
|small<B>|{d.print(small.value);d.print(*(small.view))}
<View>:<{view<&int32>}>
again:=true
empty<View><null>:'drop {
    |again|{local:1;'drop->view:&local;again=false;'drop.restart()}
}
|empty<null>|d.print("discarded")
"#,
    )
    .runs(b"42\n7\ntrue\n9\n7\nok\n255\n43\ndiscarded\n");
}

#[test]
pub fn explicit_ascription_uses_prior_narrowing_inside_matchers() {
    Case::new(
        r#"
d:@"debug"
show<null>:(enabled<boolean><null>){|enabled<boolean> && enabled~<boolean>|d.print("enabled")}
show(true)
show(false)
show(null)
<U>:<int32><null>
x<int32>:7
copy:x~<U>
|copy<int32>|d.print(copy~<int32>)
"#,
    )
    .runs(b"enabled\n7\n");
    let result =
        Case::new("bad<null>:(x<int32><null>){value:x~<int32>}").command("check", &["--json"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("E208"));
}

#[test]
pub fn type_predicates_are_boolean_values_outside_matchers() {
    Case::new(
        r#"
d:@"debug"
show<null>:(flag<boolean>){d.print(flag)}
value:7
flag:value<int32>
d.print(flag)
show(value<string>)
d.print(1+value<int32>)
row:{->n:7}
copy:row~<{n<int32>}>
d.print(copy.n)
"#,
    )
    .runs(b"true\nfalse\ntrue\n7\n");
}
