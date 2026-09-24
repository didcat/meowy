use super::{accepts, rejects};

#[test]
pub(crate) fn transitive_contents_link_loans_before_and_after_outer_borrows() {
    for source in [
        "owner:=1;cell:&owner;outer:&cell;owner=2;inner:*outer;v:*inner",
        "owner:=1;holder:{->view:&owner};owner=2;outer:&holder;inner:outer.view;v:*inner",
        "owner:=1;holder:{->view:&owner};outer:&holder;owner=2;copy:*outer;v:*(copy.view)",
        "owner:=1;cell:&owner;outer:&cell;copy:outer;owner=2;v:**copy",
        "owner:=1;cell:&owner;outer<& &int32><null>:&cell;owner=2;|outer<& &int32>|v:*(*(outer~<& &int32>))",
    ] {
        rejects(source, "E302");
    }
    accepts("owner:=1;cell:&owner;outer:&cell;v:**outer;owner=2");
    accepts("owner:1;copy:{holder:{->view:&owner};outer:&holder;->*outer};v:*(copy.view)");
}

#[test]
pub(crate) fn transitive_projection_and_pointer_reads_preserve_unrelated_owners() {
    for source in [
        "a:=1;b:=2;holder:{->left:&a;->right:&b};outer:&holder;b=3;selected:outer.left;v:*selected",
        "owner:=1;holder:{->view:&owner;->n:2};owner=3;outer:&holder;v:outer.n",
        "owner:=1;holder:{->view:&owner};outer:&holder;owner=2;same:outer==&holder",
        "owner:=1;holder:{->view<&int32><null>:&owner};outer:&holder;owner=2;|outer.view<&int32>|seen:1",
    ] {
        accepts(source);
    }
    rejects(
        "a:=1;b:=2;holder:{->left:&a;->right:&b};outer:&holder;b=3;copy:*outer",
        "E302",
    );
}

#[test]
pub(crate) fn transitive_loans_preserve_outer_and_iteration_lifetimes() {
    accepts(
        "owner:=1;i:=0;'again{cell:&owner;outer:&cell;v:**outer;owner=owner+1;i=i+1;|i<2|'again.restart()}",
    );
    rejects(
        "owner:=1;cell:&owner;outer:&cell;i:=0;'again{v:**outer;owner=2;i=i+1;|i<2|'again.restart()}",
        "E302",
    );
    rejects("owner:1;p:{cell:&owner;->&cell}", "E303");
    rejects("owner:1;result:{->cell:&owner;->outer:&cell}", "E303");
}
