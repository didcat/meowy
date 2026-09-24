use super::{accepts, rejects};

#[test]
pub(crate) fn binding_fields_mutate_without_replacing_immutable_owners() {
    accepts("object:{->field:=7;field=8};object.field=9");
    accepts("object:{->inner:{->field:=7}};object.inner.field=8;p:object.inner.&!field;*p=9");
}

#[test]
pub(crate) fn binding_fields_keep_selected_slot_replacement_permissions() {
    for source in [
        "object:{->field:=7};object={->field:=8}",
        "object:{->field:7};object.field=8",
        "object:{->inner:{->field:=7}};object.inner={->field:=8}",
        "value:7;p:&!value",
        "items:[1,2];items[1]=3",
        "items:[1,2];p:&!(items[1])",
        "object:{->items:[1,2]};object.items[1]=3",
        "object:{->items:[1,2]};p:&!(object.items[1])",
    ] {
        rejects(source, "E305");
    }
    accepts(
        "object:{->inner:{->items:=[1,2]}};object.inner.items[1]=3;p:&!(object.inner.items[2]);*p=4",
    );
    accepts("items:[{->field:=7}];items[1].field=8;p:items[1].&!field;*p=9");
    accepts(
        "items:[[{->inner:{->field:=7}}]];items[1][1].inner.field=8;p:items[1][1].inner.&!field;*p=9",
    );
}

#[test]
pub(crate) fn binding_fields_preserve_shared_access_and_physical_conflicts() {
    for source in [
        "object:{->field:=7};p:&object;object.field=8;v:p.field",
        "object:{->field:=7};p:object.&field;q:object.&!field;v:*p",
        "object:{->field:=7};p:object.&!field;object.field=8;v:*p",
    ] {
        rejects(source, "E302");
    }
    rejects("object:{->field:=7};p:&object;p.field=8", "B001");
    rejects("object:{->field:=7};p:&object;q:p.&!field", "B001");
    accepts("object:{->a:=1;->b:=2};p:object.&a;q:object.&!b;*q=3;v:*p;object.a=4");
}

#[test]
pub(crate) fn binding_fields_track_reference_updates_across_branches_and_restarts() {
    accepts("a:1;b:2;r:{->p:=&a};old:r.p;r.p=&b;v:r.*p;w:*old");
    accepts(
        "a:1;b:2;r:{->inner:{->p:=&a}};i:=0;'loop{r.inner.p=&b;v:r.inner.*p;i=i+1;|i<2|'loop.restart()}",
    );
    rejects("a:1;r:{->p:=&a};{b:2;r.p=&b};v:r.*p", "E303");
    rejects(
        "run<null>:(flag<boolean>){a:1;r:{->p:=&a};b:=2;|flag|r.p=&b;b=3;v:r.*p}",
        "E302",
    );
    accepts("run<null>:(flag<boolean>){a:1;r:{->p:=&a};b:=2;|flag|r.p=&b;r.p=&a;b=3;v:r.*p}");
}

#[test]
pub(crate) fn binding_fields_preserve_emitted_alias_snapshots_and_owner_lifetimes() {
    accepts("a:1;b:2;r:'out{'out->row:{->p:=&a};row.p=&b};v:r.row.*p");
    accepts(
        "a:1;b:2;r:'out{'out->row:{->p:=&a};i:=0;'loop{row.p=&b;i=i+1;|i<2|'loop.restart()};v:row.*p};v:r.row.*p",
    );
    accepts(
        "a:1;b:2;r:'out{'out->row:{->p:=&a};row.p=&b;i:=0;'loop{i=i+1;|i<2|'loop.restart()}};v:r.row.*p",
    );
    rejects(
        "a:1;r:'out{'out->row:{->p:=&a};{b:2;row.p=&b}};v:r.row.*p",
        "E303",
    );
    accepts(
        "<Row>:<{inner<{n<int32>:=}>}>;<R>:<{row<Row>}>;first:=true;r<R>:'out{'loop{|first|{'out->row:{->inner:{->n:=7}};p:row.inner.&!n;*p=8;first=false;'loop.restart()}}}",
    );
}

#[test]
pub(crate) fn binding_fields_keep_function_and_dispatch_copies_independent() {
    accepts(
        "<R>:<{field<int32>:=}>;f<int32>:(r<R>){r.field=8;->r.field};r<R>:{->field:=7};n:f(r);v:r.field",
    );
    accepts("r:{->field:=7};v:r.{$.field=8;->$.field};w:r.field");
    rejects("p:{r:{->field:=7};->r.&!field}", "E303");
}

#[test]
pub(crate) fn binding_fields_preserve_unchanged_allocator_alias_bounds() {
    accepts(
        "m:@\"memory\";f<m.Allocator>:(p<&int32>){->m.heap};a:1;r:{->row:{->h:f(&a);->n:=1}};v:r.row.n",
    );
    accepts(
        "m:@\"memory\";f<m.Allocator>:(p<&int32>){->m.heap};a:1;r:'out{'out->row:{->h:=m.heap;->n:=1};row.h=f(&a);i:=0;'loop{row.h=m.heap;row.n=2;i=i+1;|i<2|'loop.restart()}};v:r.row.n",
    );
    rejects(
        "m:@\"memory\";f<m.Allocator>:(p<&int32>){->m.heap};r:'out{'out->row:{->h:=m.heap};{a:1;row.h=f(&a)}};v:r.row.h",
        "E303",
    );
    accepts(
        "m:@\"memory\";f<m.Allocator>:(p<&int32>){->m.heap};r:{->h:=m.heap};{a:1;r.h=f(&a)};r.h=m.heap;v:r.h",
    );
}

#[test]
pub(crate) fn binding_fields_refresh_nullable_activity_and_forget_mutated_predicates() {
    accepts(
        "a:=1;r:{->p<&int32><null>:=null};i:=0;'loop{r.p=&a;|r.p<&int32>|{v:*(r.p~<&int32>)};r.p=null;i=i+1;|i<2|'loop.restart()};a=2",
    );
    rejects(
        "a:=1;r:{->p<&int32><null>:=null};r.p=&a;|r.p<&int32>|{old:r.p~<&int32>;r.p=null;a=2;v:*old}",
        "E302",
    );
    rejects(
        "r:{->n<int32><null>:=1;->flag:=false};p:r.&!flag;|r.n<int32>|{*p=true;v<int32>:r.n}",
        "E207",
    );
}
