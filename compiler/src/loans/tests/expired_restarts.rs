use super::{accepts, rejects};

#[test]
pub(crate) fn expired_header_values_can_be_overwritten_before_demand() {
    for source in [
        "a:1;p:=&2;i:=0;'again{p=&a;value:*p;i=i+1;|i<2|'again.restart()}",
        "a:1;p:=&a;{short:2;p=&short};i:=0;'again{p=&a;value:*p;i=i+1;|i<2|'again.restart()}",
        "a:1;p:=&a;i:=0;'again{local:i;p=&local;value:*p;i=i+1;|i<2|'again.restart()};p=&a;last:*p",
        "a:1;p:=&a;i:=0;'again{->local:2;p=&local;value:*p;i=i+1;|i<2|'again.restart()};p=&a;last:*p",
        "a:1;p:=&a;i:=0;'again{p=&a;value:*p;p=&2;i=i+1;|i<2|'again.restart()};p=&a;last:*p",
    ] {
        accepts(source);
    }
}

#[test]
pub(crate) fn same_site_storage_does_not_revive_expired_header_values() {
    for source in [
        "a:1;p:=&a;i:=0;'again{local:i;value:*p;p=&local;i=i+1;|i<2|'again.restart()}",
        "a:1;p:=&a;i:=0;'again{->local:2;value:*p;p=&local;i=i+1;|i<2|'again.restart()}",
        "a:1;p:=&a;i:=0;'again{value:*p;p=&2;i=i+1;|i<2|'again.restart()}",
        "a:1;p:=&2;i:=0;'again{value:*p;p=&a;i=i+1;|i<2|'again.restart()}",
        "a:1;p:=&a;i:=0;'again{local:i;p=&local;i=i+1;|i<2|'again.restart()};value:*p",
    ] {
        rejects(source, "E303");
    }
}

#[test]
pub(crate) fn expiring_sources_keep_current_iteration_physical_loans() {
    rejects(
        "a:1;p:=&a;i:=0;'again{local:=i;p=&local;local=3;value:*p;i=i+1;|i<2|'again.restart()}",
        "E302",
    );
    accepts(
        "a:1;p:=&a;i:=0;'again{local:=i;p=&local;value:*p;local=3;i=i+1;|i<2|'again.restart()}",
    );
    rejects(
        "a:1;p:=&a;i:=0;'again{->local:=i;p=&local;local=3;value:*p;i=i+1;|i<2|'again.restart()}",
        "E302",
    );
    rejects(
        "a:1;p:=&a;i:=0;'again{local:=i;p=&local;old:p;p=&a;local=3;value:*old;i=i+1;|i<2|'again.restart()}",
        "E302",
    );
    rejects(
        "a:1;p:=&a;i:=0;'again{local:i;cell:&p;p=&local;same:cell==&p;i=i+1;|i<2|'again.restart()}",
        "E302",
    );
}

#[test]
pub(crate) fn expiring_public_bounds_keep_current_and_old_copy_demands() {
    const PREFIX: &str = "first<&int32>:(p<&int32>,text<&string>){->p};a:1;p:=&a;i:=0;";
    accepts(&format!(
        "{PREFIX}'again{{text:=\"local\";p=first(&a,&text);value:*p;i=i+1;|i<2|'again.restart()}};p=&a;last:*p"
    ));
    rejects(
        &format!(
            "{PREFIX}'again{{text:=\"local\";p=first(&a,&text);text=\"new\";value:*p;i=i+1;|i<2|'again.restart()}}"
        ),
        "E302",
    );
    rejects(
        &format!(
            "{PREFIX}'again{{text:=\"local\";p=first(&a,&text);old:p;p=&a;text=\"new\";value:*old;i=i+1;|i<2|'again.restart()}}"
        ),
        "E302",
    );
    rejects(
        &format!(
            "{PREFIX}'again{{text:=\"local\";value:*p;p=first(&a,&text);i=i+1;|i<2|'again.restart()}}"
        ),
        "E303",
    );
}

#[test]
pub(crate) fn nested_restart_expiry_preserves_ancestor_owners_and_bounds() {
    accepts(
        "a:1;p:=&a;i:=0;'outer{local:i;p=&local;j:=0;'inner{value:*p;p=&local;j=j+1;|j<2|'inner.restart()};i=i+1;|i<2|'outer.restart()};p=&a;last:*p",
    );
    rejects(
        "a:1;p:=&a;i:=0;'outer{local:i;j:=0;'inner{value:*p;p=&local;j=j+1;|j<2|'inner.restart()};i=i+1;|i<2|'outer.restart()}",
        "E303",
    );
    accepts("value:(&7).{p:=$;i:=0;'inner{v:*p;p=$;i=i+1;|i<2|'inner.restart()};->*p}");
}

#[test]
pub(crate) fn expired_transitive_payloads_remain_lazy_and_keep_field_paths() {
    accepts(
        "<C>:<{view<&int32>;count<int32>}>;a:=1;holder<C>:(&2).{->view:$;->count:3};p:=&holder;i:=0;'again{a=4;value:p.count;p=&holder;i=i+1;|i<2|'again.restart()}",
    );
    rejects(
        "<C>:<{view<&int32>;count<int32>}>;holder<C>:(&2).{->view:$;->count:3};p:=&holder;i:=0;'again{value:*(p.view);p=&holder;i=i+1;|i<2|'again.restart()}",
        "E303",
    );
    accepts(
        "a:=1;holder:(&2).{->old:$;->live:&a};p:=&holder;i:=0;'again{value:*(p.live);p=&holder;i=i+1;|i<2|'again.restart()}",
    );
    rejects(
        "a:=1;holder:(&2).{->old:$;->live:&a};p:=&holder;i:=0;'again{a=3;value:*(p.live);p=&holder;i=i+1;|i<2|'again.restart()}",
        "E302",
    );
}
