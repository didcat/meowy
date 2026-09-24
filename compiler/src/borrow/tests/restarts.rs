use super::{accepts, rejects};

#[test]
pub(crate) fn restart_headers_reach_all_carried_sources_before_validation() {
    for source in [
        "a:1;b:2;p:=&a;count:=0;'loop{p=&b;count=count+1;|count<2|'loop.restart()};value:*p",
        "a:1;b:2;p:=&a;q:=&a;r:=&a;count:=0;'loop{p=q;q=r;r=&b;count=count+1;|count<4|'loop.restart()};value:*p",
        "a:1;b:2;p:=&a;outer:=0;'outer{inner:=0;'inner{p=&b;inner=inner+1;|inner<2|'inner.restart()};outer=outer+1;|outer<2|'outer.restart()};value:*p",
    ] {
        accepts(source);
    }
    rejects(
        "a:1;b:=2;p:=&a;q:=&a;r:=&a;count:=0;'loop{p=q;q=r;r=&b;count=count+1;|count<4|'loop.restart()};b=3;value:*p",
        "E302",
    );
    rejects(
        "a:=1;b:2;p:=&b;later:=false;count:=0;'loop{|later|{a=3;value:*p};|!later|p=&a;later=true;count=count+1;|count<2|'loop.restart()}",
        "E302",
    );
}

#[test]
pub(crate) fn restart_domains_preserve_unread_expired_sources_and_bounds() {
    for source in [
        "a:1;p:=&a;'loop{local:2;p=&local;'loop.restart()}",
        "a:1;p:=&a;'loop{->n:2;p=&n;'loop.restart()}",
        "a:1;p:=&a;'loop{p=&1;'loop.restart()}",
        "first<&int32>:(p<&int32>,other<&string>){->p};a:1;p:=&a;'loop{local:\"short\";p=first(&a,&local);'loop.restart()}",
    ] {
        accepts(source);
    }
    accepts(
        "a:1;p:=&a;count:=0;'loop{local:2;p=&local;value:*p;p=&a;count=count+1;|count<2|'loop.restart()};value:*p",
    );
}

#[test]
pub(crate) fn expired_header_sources_do_not_revive_after_same_site_initialization() {
    for source in [
        "a:1;p:=&a;'loop{local:2;value:*p;p=&local;'loop.restart()}",
        "a:1;p:=&a;'loop{->n:2;value:*p;p=&n;'loop.restart()}",
        "a:1;p:=&a;'loop{value:(&2).{->*p};p=&3;'loop.restart()}",
        "first<&int32>:(p<&int32>,other<&string>){->p};a:1;p:=&a;'loop{local:\"short\";value:*p;p=first(&a,&local);'loop.restart()}",
        "a:1;holder:{->n:2};p:=&holder;'loop{local:{->n:3};field:&(p.n);p=&local;'loop.restart()}",
        "a:1;p:=&a;'loop{local:2;old:p;p=&local;value:*old;'loop.restart()}",
    ] {
        rejects(source, "E303");
    }
}

#[test]
pub(crate) fn expired_entry_sources_can_be_replaced_before_reading() {
    for source in [
        "a:1;p:=&2;i:=0;'loop{p=&a;value:*p;i=i+1;|i<2|'loop.restart()}",
        "a:1;p:=&a;'out{local:2;p=&local};i:=0;'loop{p=&a;value:*p;i=i+1;|i<2|'loop.restart()}",
        "a:1;p:=&a;'out{->n:2;p=&n};i:=0;'loop{p=&a;value:*p;i=i+1;|i<2|'loop.restart()}",
        "value:(&7).{p:=$;i:=0;'inner{v:*p;p=$;i=i+1;|i<2|'inner.restart()};->*p}",
    ] {
        accepts(source);
    }
}

#[test]
pub(crate) fn expired_transitive_headers_validate_only_demanded_parts() {
    let source = "<C>:<{view<&int32><null>;n<int32>}>;cell<C>:(&7).{->view:$;->n:4};p:=&cell;i:=0;";
    accepts(&format!(
        "{source}'loop{{n:p.n;|p.view<null>|{{}};p=&cell;i=i+1;|i<2|'loop.restart()}}"
    ));
    rejects(
        &format!("{source}'loop{{value:p.view;p=&cell;i=i+1;|i<2|'loop.restart()}}"),
        "E303",
    );
    rejects(
        &format!("{source}'loop{{copy:*p;p=&cell;i=i+1;|i<2|'loop.restart()}}"),
        "E303",
    );
}

#[test]
pub(crate) fn restarted_copies_keep_initial_storage_and_scoped_exits() {
    for source in [
        "a:=1;b:=2;p:=&a;old:p;count:=0;'loop{p=&b;count=count+1;|count<2|'loop.restart()};value:*old;a=3;other:*p",
        "a:1;b:2;c:3;p:=&a;count:=0;'loop{count=count+1;p={p=&b;|count<2|'loop.restart();->&c}};value:*p",
        "a:1;b:2;p:=&a;count:=0;'loop{p=&b;count=count+1;|count<2|'loop.restart();'loop.leave()};value:*p",
    ] {
        accepts(source);
    }
    rejects(
        "a:=1;b:2;p:=&a;old:p;count:=0;'loop{p=&b;count=count+1;|count<2|'loop.restart()};a=3;value:*old",
        "E302",
    );
}

#[test]
pub(crate) fn restart_replay_has_a_bounded_public_source_fixed_point() {
    let mut source = "a:1;b:2;".to_owned();
    for index in 0..80 {
        source.push_str(&format!("p{index}:=&a;"));
    }
    source.push_str("'loop{");
    for index in 0..79 {
        source.push_str(&format!("p{index}=p{};", index + 1));
    }
    source.push_str("p79=&b;'loop.restart()}");
    let errors = crate::compile(&source).unwrap_err();
    assert_eq!(errors[0].code, "B001", "{errors:?}");
    assert!(errors[0].message.contains("budget"));
}
