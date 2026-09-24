use super::Case;
use super::exclusive_references::rejects;

#[test]
pub(crate) fn binding_fields_mutate_owned_fields_without_replacing_the_binding() {
    Case::new(
        r#"
d:@"debug"
object:{->field:=7;field=8;->inner:{->field:=10}}
d.print(object.field)
object.field=9
object.inner.field=11
old:object
p:object.inner.&!field
*p=12
d.print(object.field)
d.print(object.inner.field)
d.print(old.inner.field)
"#,
    )
    .runs(b"8\n9\n12\n11\n");
}

#[test]
pub(crate) fn binding_fields_preserve_list_slots_and_mutable_record_elements() {
    Case::new(
        r#"
d:@"debug"
object:{->inner:{->items:=[1,2]}}
object.inner.items[1]=3
p:&!(object.inner.items[2])
*p=4
rows:[{->inner:{->n:=5};->locked:6}]
rows[1].inner.n=7
q:rows[1].inner.&!n
*q=8
nested:[[{->n:=9}]]
nested[1][1].n=10
d.print(object.inner.items[1])
d.print(object.inner.items[2])
d.print(rows[1].inner.n)
d.print(rows[1].locked)
d.print(nested[1][1].n)
"#,
    )
    .runs(b"3\n4\n8\n6\n10\n");
}

#[test]
pub(crate) fn binding_fields_preserve_reference_versions_and_old_copies() {
    Case::new(
        r#"
d:@"debug"
run<null>:(flag<boolean>){
    a:1;b:2
    r:{->inner:{->p:=&a}}
    old:r.inner.p
    |flag|r.inner.p=&b
    d.print(r.inner.*p)
    d.print(*old)
    i:=0
    'loop{
        r.inner.p=&b
        d.print(r.inner.*p)
        i=i+1
        |i<2|'loop.restart()
    }
}
run(false)
run(true)
"#,
    )
    .runs(b"1\n1\n2\n2\n2\n1\n2\n2\n");
}

#[test]
pub(crate) fn binding_fields_update_emitted_aliases_and_carried_records() {
    Case::new(
        r#"
d:@"debug"
a:1;b:2
r:'out{
    'out->row:{->p:=&a}
    old:row.p
    i:=0
    'loop{row.p=&b;i=i+1;|i<2|'loop.restart()}
    d.print(*old)
}
d.print(r.row.*p)
<Row>:<{inner<{n<int32>:=}>}>
<R>:<{row<Row>}>
first:=true
carried<R>:'out{'loop{|first|{
    'out->row:{->inner:{->n:=7}}
    p:row.inner.&!n
    *p=8
    first=false
    'loop.restart()
}}}
d.print(carried.row.inner.n)
"#,
    )
    .runs(b"1\n2\n8\n");
}

#[test]
pub(crate) fn binding_fields_keep_function_dispatch_and_captured_store_semantics() {
    Case::new(
        r#"
d:@"debug"
<R>:<{n<int32>:=;other<int32>:=}>
change<int32>:(r<R>){r.n=8;->r.n}
r<R>:{->n:=7;->other:=1}
d.print(change(r))
d.print(r.n)
value:r.{$.n=9;->$.n}
d.print(value)
d.print(r.n)
p:=r.&!n
*p={p=r.&!other;->10}
d.print(r.n)
d.print(*p)
"#,
    )
    .runs(b"8\n7\n9\n7\n10\n1\n");
}

#[test]
pub(crate) fn binding_fields_preserve_allocator_bound_aliases() {
    Case::new(
        r#"
d:@"debug"
m:@"memory"
factory<m.Allocator>:(p<&int32>){->m.heap}
a:1
unchanged:{->row:{->h:factory(&a);->n:=1}}
d.print(unchanged.row.n)
r:'out{
    'out->row:{->h:=m.heap;->n:=1}
    row.h=factory(&a)
    i:=0
    'loop{row.h=m.heap;row.n=2;i=i+1;|i<2|'loop.restart()}
}

d.print(r.row.n)
"#,
    )
    .runs(b"1\n2\n");
    rejects(
        "m:@\"memory\";f<m.Allocator>:(p<&int32>){->m.heap};r:'out{'out->row:{->h:=m.heap};{a:1;row.h=f(&a)}};v:r.row.h",
        "E303",
    );
}

#[test]
pub(crate) fn binding_fields_reject_root_replacement_immutable_slots_and_shared_conflicts() {
    for source in [
        "r:{->n:=1};r={->n:=2}",
        "r:{->inner:{->n:=1}};r.inner={->n:=2}",
        "r:{->n:1};r.n=2",
        "n:1;p:&!n",
        "xs:[1];xs[1]=2",
        "xs:[1];p:&!(xs[1])",
        "r:{->xs:[1]};r.xs[1]=2",
        "rows:[{->n:=1}];rows[1]={->n:=2}",
        "r:{->n:=1};p:&r;*(p.&n)=2",
    ] {
        rejects(source, "E305");
    }
    for source in [
        "r:{->n:=1};p:&r;r.n=2;v:p.n",
        "r:{->n:=1};p:r.&n;q:r.&!n;v:*p",
        "r:{->n:=1};p:r.&!n;r.n=2;v:*p",
        "run<null>:(flag<boolean>){a:1;r:{->p:=&a};b:=2;|flag|r.p=&b;b=3;v:r.*p}",
    ] {
        rejects(source, "E302");
    }
    for source in [
        "r:{->n:=1};p:&r;p.n=2",
        "r:{->n:=1};p:&r;q:p.&!n",
        "r:{->n:=1};p:&!r",
    ] {
        rejects(source, "B001");
    }
    rejects("p:{r:{->n:=1};->r.&!n}", "E303");
    rejects("a:1;r:{->p:=&a};{b:2;r.p=&b};v:r.*p", "E303");
    rejects("r:{->n:=1};p:r.&!n;q:p;v:*p;w:*q", "E301");
}

#[test]
pub(crate) fn binding_fields_refresh_nullable_reference_activity() {
    Case::new(
        r#"
d:@"debug"
a:=1
r:{->p<&int32><null>:=null}
i:=0
'loop{
    r.p=&a
    |r.p<&int32>|d.print(*(r.p<&int32>))
    r.p=null
    i=i+1
    |i<2|'loop.restart()
}
a=3
d.print(a)
"#,
    )
    .runs(b"1\n1\n3\n");
    rejects(
        "a:=1;r:{->p<&int32><null>:=null};r.p=&a;|r.p<&int32>|{old:r.p<&int32>;r.p=null;a=2;v:*old}",
        "E302",
    );
    rejects(
        "r:{->n<int32><null>:=1;->flag:=false};p:r.&!flag;|r.n<int32>|{*p=true;v<int32>:r.n}",
        "E207",
    );
}
