use super::{accepts, rejects};

#[test]
pub(crate) fn temporary_copy_owners_live_through_their_complete_statement() {
    for source in [
        "value:*(&(1+2))",
        "value:*(&{->1})",
        "value:*(&([1,2][2]))",
        "value:*(&({->row:{->n:4}}.row.n))",
        "id<&int32>:(p<&int32>){->p};value:*(id(&(1+2)))",
        "read<uint8>:(p<&uint8>){->*p};byte<uint8>:1;value:read(&(byte<uint8>))",
        "read<uint8>:(p<&uint8>){->*p};value:read(&{byte<uint8>:1;->byte})",
        "first<&int32>:(p<&int32[2]>){->&(p[1])};value:*(first(&[3,4]))",
        "value:(&{->n:4}).{->$.n}",
        "|*(&true)|value:*(&1)",
        "owner:=1;view:&owner;same:&(*view+1)==&{owner=3;->2}",
    ] {
        accepts(source);
    }
}

#[test]
pub(crate) fn temporary_borrows_do_not_extend_past_inner_statement_ends() {
    for source in [
        "view:&1;value:*view",
        "view:&(1+2);value:*view",
        "view:&([1,2][1]);value:*view",
        "value:*({->&{->1}})",
        "value:*({p:&1;->p})",
        "value:{->view:&{->1}}",
        "id<&int32>:(p<&int32>){->p};view:id(&1);value:*view",
        "f<&int32>:(){->&(1+2)}",
    ] {
        rejects(source, "E303");
    }
}

#[test]
pub(crate) fn temporary_materialization_keeps_reference_and_checked_index_boundaries() {
    rejects("value:&!(1+2)", "B001");
    rejects("value:*(&([1][2]))", "E101");
    rejects("read<uint8>:(p<&uint8>){->*p};value:read(&1)", "E212");
    rejects("view<&uint8>:&1", "E207");
    accepts("owner:{->n:1};value:*(&({->view:&owner}.view.n))");
    rejects("owner:=1;view:&owner;same:&*view==&{owner=3;->2}", "E302");
}

#[test]
pub(crate) fn nonreturning_initializers_do_not_create_temporary_owners() {
    for source in [
        "d:@\"debug\";value:&{d.panic(\"stop\")}",
        "value:'out{->*(&{'out->7;'out.leave()})}",
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};value:&(stop())",
    ] {
        accepts(source);
    }
}

#[test]
pub(crate) fn call_entry_validates_every_active_transitive_argument_origin() {
    for source in [
        "read<int32>:(p<& &int32>){->**p};cell:&1;value:read(&cell)",
        "read<int32>:(p<& & &int32>){->***p};cell:&1;outer:&cell;value:read(&outer)",
        "keep<&int32>:(p<&int32>,q<&string>){->p};read<int32>:(p<& &int32>){->**p};owner:1;cell:keep(&owner,&\"x\");value:read(&cell)",
        "<C>:<{view<&int32>;n<int32>}>;read<int32>:(p<&C>){->*(p.view)};holder:(&1).{->view:$;->n:2};value:read(&holder)",
        "<V>:<&int32><null>;read<int32>:(p<&V>){->0};f<null>:(flag<boolean>){cell<V>:(&1).{|flag|->$};|flag|value:read(&cell)}",
    ] {
        rejects(source, "E303");
    }
    for source in [
        "read<int32>:(p<& &int32>,n<int32>){->**p};cell:&1;value:'out{->read(&cell,{'out->7;'out.leave()})}",
        "d:@\"debug\";read<int32>:(p<& &int32>,n<int32>){->**p};cell:&1;value:read(&cell,{d.panic(\"stop\")})",
        "read<int32>:(p<& &int32>){->**p};cell:&1;|false|value:read(&cell)",
        "<V>:<&int32><null>;read<int32>:(p<&V>){->0};cell<V>:null;value:read(&cell)",
        "<V>:<&int32><null>;read<int32>:(p<&V>){->0};cell<V>:{|false|->&1};value:read(&cell)",
        "read<int32>:(p<& &int32>){->**p};flag:false;cell:&1;|flag|value:read(&cell)",
        "holder:(&1).{->view:$;->n:2};p:&holder;value:p.n",
        "<V>:<&int32><null>;read<int32>:(p<&V>){->0};f<null>:(flag<boolean>){cell<V>:(&1).{|flag|->$};|!flag|value:read(&cell)}",
    ] {
        accepts(source);
    }
}

#[test]
pub(crate) fn tag_inspection_skips_payload_origins_but_checks_accessed_storage() {
    for source in [
        "holder:(&1).{->view<&int32><null>:$;->n:2};p:&holder;|p.view<&int32>|v:1",
        "holder:(&1).{->view<&int32><null>:$;->n:2};p:&holder;q:&p;|(*q).view<null>|v:1",
        "r:&1;|r<&int32>|v:1",
        "r:&1;p:&r;|(*p)<&int32>|v:1",
        "<V>:<&int32><null>;read<int32>:(p<&V>){->0};cell<V>:&1;|cell<null>|value:read(&cell)",
        "d:@\"debug\";|{d.print(\"before\");->1}<int32>|v:1",
    ] {
        accepts(source);
    }
    for source in [
        "r:&1;value:r",
        "r:&1;|(*r)<int32>|v:1",
        "p:&{->n:1};|p.n<int32>|v:1",
        "p:&{->tag<int32><null>:null};|p.tag<int32>|v:1",
        "r:&1;p:&r;value:*p",
        "d:@\"debug\";r:&1;|{d.print(\"before\");->r}<&int32>|v:1",
    ] {
        rejects(source, "E303");
    }
}
