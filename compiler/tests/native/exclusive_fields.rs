use super::Case;
use super::exclusive_references::rejects;

#[test]
pub fn scalar_field_stores_preserve_types_layout_and_siblings() {
    Case::new(
        r#"
d:@"debug"
v:={->n<int8>:=126;->flag:=true;->f<float32>:=1.5;->label:"kept"}
p:&!(v.n);q:&!(v.flag);r:&!(v.f)
*p=*p+1;*q=false;*r=2.5
d.print(*p);d.print(*q);d.print(*r);d.print(v.label)
"#,
    )
    .runs(b"127\nfalse\n2.5\nkept\n");
}

#[test]
pub fn sibling_field_loans_and_direct_accesses_are_disjoint() {
    Case::new(
        r#"
d:@"debug"
v:={->a:=1;->b:=2;->c:=3}
p:&!(v.a);q:&!(v.b);s:&(v.c)
*p=4;*q=5;d.print(*s);d.print(*p);d.print(*q)
v.c=6;d.print(v.c)
"#,
    )
    .runs(b"3\n4\n5\n6\n");
}

#[test]
pub fn nested_fields_keep_each_mutable_boundary_and_projection() {
    Case::new("v:{->n:=1};p:&!(v.n)").runs(b"");
    Case::new("v:={->inner:{->n:=1}};p:&!(v.inner.n)").runs(b"");
    Case::new("<R>:<{n<int32>:=}>;f<null>:(v<R>){p:&!(v.n)}").runs(b"");
    Case::new("v:={->n:=1};v.{p:&!($.n)}").runs(b"");
    Case::new(
        r#"
d:@"debug"
v:={->inner:={->n:=1;->other:=2};->tail:=3}
p:&!(((v).inner).n)
v.inner.other=4;v.tail=5;*p=6
d.print(v.inner.other);d.print(v.tail);d.print(*p)
"#,
    )
    .runs(b"4\n5\n6\n");
    for source in [
        "v:={->n:1};p:&!(v.n)",
        "v:={->inner:={->n:1}};p:&!(v.inner.n)",
    ] {
        rejects(source, "E305");
    }
}

#[test]
pub fn primary_reads_do_not_access_named_field_storage() {
    Case::new(
        r#"
d:@"debug"
v:={->1;->inner:={->2;->n:=3};->tail:=4}
p:&!(v.inner.n);q:&!(v.tail)
a<int32>:v;b<int32>:v.inner
d.print(a);d.print(b);*p=5;*q=6;d.print(*p);d.print(*q)
"#,
    )
    .runs(b"1\n2\n5\n6\n");
}

#[test]
pub fn overlapping_fields_ancestors_and_whole_owners_conflict() {
    for source in [
        "v:={->n:=1};p:&!(v.n);x:v.n;w:*p",
        "v:={->n:=1};p:&!(v.n);v.n=2;w:*p",
        "v:={->n:=1};p:&!(v.n);c:v;w:*p",
        "v:={->n:=1};p:&!(v.n);v={->n:=2};w:*p",
        "v:={->inner:={->n:=1}};p:&!(v.inner.n);c:v.inner;w:*p",
        "v:={->inner:={->n:=1}};p:&!(v.inner.n);v.inner={->n:=2};w:*p",
        "v:={->n:=1};s:&v;p:&!(v.n);w:s.n",
        "v:={->inner:={->n:=1}};s:&(v.inner);p:&!(v.inner.n);w:s.n",
        "v:={->n:=1};s:&(v.n);p:&!(v.n);w:*s",
        "v:={->n:=1};p:&!(v.n);s:&(v.n);w:*p",
    ] {
        rejects(source, "E302");
    }
}

#[test]
pub fn copied_records_have_independent_borrowed_storage() {
    Case::new(
        r#"
d:@"debug"
v:={->n:=1};copy:=v;p:&!(copy.n);v.n=2;*p=3
d.print(v.n);d.print(*p)
"#,
    )
    .runs(b"2\n3\n");
}

#[test]
pub fn field_reference_moves_and_reborrows_preserve_authority() {
    Case::new(
        r#"
d:@"debug"
v:={->a:=1;->b:=2};p:=&!(v.a);q:p;p=&!(v.b)
s:&*q;d.print(*s);*q=3;r:&!*p;*r=4;d.print(*r);*p=5
d.print(*q);d.print(*p)
"#,
    )
    .runs(b"1\n4\n3\n5\n");
    rejects("v:={->n:=1};p:&!(v.n);q:p;w:*p", "E301");
    rejects("v:={->n:=1};p:&!(v.n);s:&*p;*p=2;w:*s", "E302");
    rejects("v:={->n:=1};p:&!(v.n);q:&!*p;w:*p;z:*q", "E302");
}

#[test]
pub fn function_and_block_transfers_keep_original_field_addresses() {
    Case::new(
        r#"
d:@"debug"
id<&!int32>:(p<&!int32>){->p}
swap<null>:(p<&!int32>,q<&!int32>){v:*p;*p=*q;*q=v}
v:={->a:=1;->b:=2};swap(&!(v.a),&!(v.b))
p:{->id(&!(v.a))};*p=3;d.print(*p);d.print(v.b)
"#,
    )
    .runs(b"3\n1\n");
}

#[test]
pub fn field_result_bounds_keep_unrelated_input_write_protection() {
    Case::new(r#"d:@"debug";first<&!int32>:(p<&!int32>,b<&boolean>){->p};v:={->n:=1;->flag:=true};r:first(&!(v.n),&(v.flag));d.print(v.flag);*r=2;d.print(*r);v.flag=false;d.print(v.flag)"#).runs(b"true\n2\nfalse\n");
    rejects(
        "first<&!int32>:(p<&!int32>,b<&boolean>){->p};v:={->n:=1;->flag:=true};r:first(&!(v.n),&(v.flag));v.flag=false;w:*r",
        "E302",
    );
}

#[test]
pub fn guarded_field_choices_preserve_disjoint_owners() {
    Case::new(
        r#"
d:@"debug"
f<null>:(flag<boolean>){
 v:={->a:=1;->b:=2};p:{|flag|->&!(v.a);|!flag|->&!(v.b)}
 |flag|v.b=3
 |!flag|v.a=4
 *p=5;d.print(*p);d.print(v.a);d.print(v.b)
}
f(true);f(false)
"#,
    )
    .runs(b"5\n5\n3\n5\n4\n5\n");
}

#[test]
pub fn indirect_stores_capture_fields_once_and_skip_cancelled_writes() {
    Case::new(
        r#"
d:@"debug"
v:={->a:=1;->b:=2};p:=&!(v.a)
*p={v.b=3;p=&!(v.b);->4};d.print(v.a);d.print(*p)
'out{*p={v={->a:=5;->b:=6};'out.leave()}}
d.print(v.a);d.print(v.b)
"#,
    )
    .runs(b"4\n3\n5\n6\n");
    rejects(
        "v:={->a:=1;->b:=2};p:&!(v.a);*p={v={->a:=3;->b:=4};->5}",
        "E302",
    );
}

#[test]
pub fn field_owners_cannot_escape_scope_or_function_lifetimes() {
    rejects("p:{v:={->n:=1};->&!(v.n)}", "E303");
    rejects("bad<&!int32>:(){v:={->n:=1};->&!(v.n)}", "E303");
    rejects("a:=1;p:=&!a;{v:={->n:=2};p=&!(v.n)};w:*p", "E303");
}

#[test]
pub fn unsupported_roots_pointees_and_unknown_fields_stay_explicit() {
    for source in [
        "v:={->n:=1};p:&!v",
        "v:={->inner:={->n:=1}};p:&!(v.inner)",
        "v:={->label:=\"x\"};p:&!(v.label)",
        "v:={->n:=1};s:&v;p:&!(s.n)",
        "v:={->n:=1};s:&v;p:&!((*s).n)",
        "p:&!(({->n:=1}).n)",
        "r:{->row:={->n:=1};p:&!row}",
        "<R>:<{n<int32>:=}>;v<R><null>:={->n:=1};|v<R>|{p:&!(v.n)}",
        "v:={->n:=1};p:&!(v.n);'again{'again.restart()}",
    ] {
        rejects(source, "B001");
    }
    rejects("v:={->n:=1};p:&!(v.missing)", "E201");
}

#[test]
pub fn deep_field_paths_preserve_existing_structural_budgets() {
    for (depth, code) in [(16, None), (300, Some("B001"))] {
        let mut value = String::from("{->n:=1}");
        for _ in 0..depth {
            value = format!("{{->inner:={value}}}");
        }
        let path = format!("v{}.n", ".inner".repeat(depth));
        let source = format!("d:@\"debug\";v:={value};p:&!({path});*p=2;d.print(*p)");
        if let Some(code) = code {
            rejects(&source, code);
        } else {
            Case::new(&source).runs(b"2\n");
        }
    }
}
