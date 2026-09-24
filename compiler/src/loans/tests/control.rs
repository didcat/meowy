use super::{accepts, rejects};

#[test]
pub(crate) fn missing_reborrow_snapshots_require_proven_unreachability() {
    for source in [
        "a:[1,2];|false|{r:&(a[1])}",
        "a:[1,2];x:true||{r:&(a[1]);->true}",
        "a:[1,2];x:false&&{r:&(a[1]);->true}",
        "a:=[1,2];|false|{r:&(a[{a=[3,4];->99}])}",
        "a:{->value:1};p:&a;|false|{r:&(p.value)}",
    ] {
        accepts(source);
    }
    rejects("a:=[1,2];|true|{r:&(a[1]);a=[3,4];value:*r}", "E302");
    let program = crate::compile("a:[1,2];r:&(a[1])").unwrap();
    let error = super::Graph::new(
        &program,
        &super::Facts::default(),
        &super::Proofs::default(),
        &mut super::Flow::new(),
    )
    .check(&program.body, &[])
    .unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("missing shared reborrow proof"));
}

#[test]
pub(crate) fn expression_temporaries_keep_borrows_until_their_consumption() {
    accepts("a:=1;r:&a;v:*r+{a=2;->1}");
    accepts("a:=1;r:&a;v:*r=={a=2;->2}");
    rejects("a:=1;b:2;r:&a;same:r=={a=2;->&b}", "E302");
    rejects("a:=1;b:2;same:&a=={a=2;->&b}", "E302");
    rejects("a:=1;r:{->&a;a=2};v:*r", "E302");
}

#[test]
pub(crate) fn guard_proofs_exclude_disjoint_writes_and_uses() {
    accepts("f<null>:(flag<boolean>){a:=1;r:&a;|flag|a=2;|!flag|v:*r}");
    accepts(
        "f<int32>:(flag<boolean>){a:=11;b:=22;r:{|flag|->&a;|!flag|->&b};|flag|b=33;|!flag|a=44;->*r}",
    );
    rejects(
        "f<int32>:(flag<boolean>){a:=11;b:=22;r:{|flag|->&a;|!flag|->&b};|flag|a=33;->*r}",
        "E302",
    );
    accepts("a:=1;r:&a;v:false&&{a=2;->true};x:*r");
    accepts("a:=1;r:&a;v:true||{a=2;->true};x:*r");
}

#[test]
pub(crate) fn restart_solves_liveness_across_iterations() {
    accepts("a:=0;i:=0;'loop {r:&a;v:*r;a=a+1;i=i+1;|i<2|'loop.restart()}");
    rejects(
        "a:=1;r:&a;later:=false;i:=0;'loop {|later|v:*r;|!later|a=2;later=true;i=i+1;|i<2|'loop.restart()}",
        "E302",
    );
    accepts("a:=1;again:=true;r:'out {|again|{'out->&a;a=2;again=false;'out.restart()};->&a};v:*r");
    rejects(
        "a:=1;r:&a;i:=0;'loop {v:*r;a=2;i=i+1;|i<2|'loop.restart()}",
        "E302",
    );
}

#[test]
pub(crate) fn scoped_exits_end_unreachable_loan_uses() {
    accepts("a:=1;r:&a;'out {'out.leave();a=2;v:*r}");
    accepts("a:=1;r:&a;v:*'out {|true|{'out->r;'out.leave()};a=2;->r}");
    rejects("a:=1;r:&a;v:*'out {a=2;->r}", "E302");
    accepts("f<int32>:(){a:=1;r:&a;a=*r+1;->a};v:f()");
}

#[test]
pub(crate) fn component_origins_keep_branch_and_iteration_liveness() {
    accepts(
        "f<int32>:(flag<boolean>){a:=1;b:=2;r:{|flag|->view:&a;|!flag|->view:&b};|flag|b=3;|!flag|a=4;->*(r.view)}",
    );
    rejects(
        "f<int32>:(flag<boolean>){a:=1;b:=2;r:{|flag|->view:&a;|!flag|->view:&b};|flag|a=3;->*(r.view)}",
        "E302",
    );
    accepts("a:=0;i:=0;'loop {r:{->view:&a};x:*(r.view);a=a+1;i=i+1;|i<2|'loop.restart()}");
    rejects(
        "a:=1;r:{->view:&a};i:=0;'loop {x:*(r.view);a=2;i=i+1;|i<2|'loop.restart()}",
        "E302",
    );
}

#[test]
pub(crate) fn union_activity_assumptions_are_reestablished_after_restart() {
    accepts(
        "a:=0;i:=0;'loop {r<&int32><null>:{|i<2|->&a};|r<&int32>|{x:*(r~<&int32>)};a=a+1;i=i+1;|i<3|'loop.restart()}",
    );
    rejects(
        "a:=1;r<&int32><null>:&a;first:=true;i:=0;'loop {|!first&&r<&int32>|{x:*(r~<&int32>)};|first|a=2;first=false;i=i+1;|i<2|'loop.restart()}",
        "E302",
    );
}
