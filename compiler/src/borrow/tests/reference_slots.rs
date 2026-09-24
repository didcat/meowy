use super::{accepts, rejects};

#[test]
pub(crate) fn reference_alias_copies_and_reborrows_keep_pointee_origins() {
    for source in [
        "owner:1;result:{->view:&owner;->view};p<&int32>:result;v:*p",
        "owner:{->n:1};result:{->carrier:{->view:&owner};->&(carrier.view.n)};p<&int32>:result;v:*p",
        "id<&int32>:(p<&int32>){result:{->view:p;->view};->result}",
        "owner:1;result:{->view<&int32><null>:&owner;|view<&int32>|{p:&(*(view~<&int32>));v:*p}}",
    ] {
        accepts(source);
    }
    rejects("p:{owner:1;->view:&owner;->view}", "E303");
    rejects(
        "first<&int32>:(p<&int32>,q<&string>){->p};wrap<&int32>:(p<&int32>){local:\"x\";result:{->view:first(p,&local);->view};->result}",
        "E303",
    );
}

#[test]
pub(crate) fn reference_carrier_field_addresses_follow_their_storage_owner() {
    for source in [
        "owner:1;result:'out{p:{'out->carrier:{->view:&owner;->n:2};->&(carrier.n)};->seen:*p}",
        "owner:1;result:'out{p:{'out->carrier:{->view:&owner;->items:[1,2]};->&(carrier.items[2])};->seen:*p}",
        "d:@\"debug\";'done{result:'out{p:{owner:1;'out->carrier:{->view:&owner;->n:2};->&(carrier.n)};d.print(*p);'done.leave()}}",
        "f<int32>:(carrier<{view<&int32>;n<int32>}>){p:&(carrier.n);->*p}",
        "owner:1;carrier:{->view:&owner;->n:2};copy:carrier;p:&(copy.n);v:*p",
        "f<null>:(owner<&int32>,flag<boolean>){result:'out{|flag|{'out->carrier:{->view:owner;->n:2};p:&(carrier.n);v:*p};|!flag|{'out->carrier:\"x\"}}}",
    ] {
        accepts(source);
    }
    for source in [
        "owner:1;result:{->carrier:{->view:&owner;->n:2};->saved:&(carrier.n)}",
        "owner:1;p:{carrier:{->view:&owner;->n:2};->&(carrier.n)}",
        "f<&int32>:(carrier<{view<&int32>;n<int32>}>){->&(carrier.n)}",
    ] {
        rejects(source, "E303");
    }
}

#[test]
pub(crate) fn selected_carrier_addresses_keep_mutation_and_union_storage_boundaries() {
    accepts(
        "owner:1;result:{->carrier:={->view:&owner;->n:2};carrier={->view:&owner;->n:3};n:=2;'loop{n=n-1;|n>0|'loop.restart()}}",
    );
    rejects(
        "f<null>:(owner<&int32>,flag<boolean>){result:'out{|flag|{'out->carrier<{view<&int32>;n<int32>}><null>:{->view:owner;->n:2};|carrier<{view<&int32>;n<int32>}>|{p:&(carrier.n)}};|!flag|{'out->carrier:\"x\"}}}",
        "B001",
    );
}
