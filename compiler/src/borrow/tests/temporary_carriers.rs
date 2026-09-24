use super::{accepts, rejects};

#[test]
pub(crate) fn temporary_dereference_copies_keep_original_pointee_sources() {
    for source in [
        "owner:1;copy:*(&(&owner));value:*copy",
        "owner:1;copy:*(&{->view:&owner;->n:2});value:*(copy.view)",
        "owner:1;value:*(&({->view:&owner;->n:2}.n))",
        "owner:{->n:1};copy:&({->view:&owner}.view.n);value:*copy",
        "owner:1;copy:{->*(&{->view:&owner})};value:*(copy.view)",
        "owner:1;copy:(&(&owner)).{->*$};value:*copy",
        "owner:1;value:***(&(&(&owner)))",
        "load<&int32>:(p<& &int32>){->*p};owner:1;value:*(load(&(&owner)))",
    ] {
        accepts(source);
    }
}

#[test]
pub(crate) fn temporary_cell_origins_and_public_bounds_do_not_escape() {
    for source in [
        "owner:1;cell:&(&owner);value:**cell",
        "owner:1;cell:&{->view:&owner;->n:2};value:cell.n",
        "owner:1;cell:&({->view:&owner;->n:2}.n);value:*cell",
        "owner:1;copy:*(&(&(&owner)));later:**copy",
        "load<&int32>:(p<& &int32>){->*p};owner:1;copy:load(&(&owner));value:*copy",
        "owner:1;copy:*({->&(&owner)})",
        "copy:*(&{owner:1;->view:&owner})",
    ] {
        rejects(source, "E303");
    }
}

#[test]
pub(crate) fn temporary_snapshots_keep_active_loans_without_reading_unused_pointees() {
    for source in [
        "owner:=1;copy:*(&{->view<&int32><null>:null;->n:2});owner=3;|copy.view<&int32>|value:*(copy.view<&int32>)",
        "holder:(&1).{->view:$;->n:2};value:(*(&(&holder))).n",
        "owner:=1;copy:*(&{->view:&owner});value:*(copy.view);owner=3",
    ] {
        accepts(source);
    }
    for source in [
        "owner:=1;copy:*(&{->view:&owner});owner=3;value:*(copy.view)",
        "owner:=1;view:&owner;value:*(&({owner=3;->view:view;->n:2}.n))",
        "owner:=1;cell:&owner;copy:*(&cell);owner=3;value:*copy",
    ] {
        rejects(source, "E302");
    }
}
