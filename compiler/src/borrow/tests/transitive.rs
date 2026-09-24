use super::{accepts, rejects};

#[test]
pub(crate) fn dereference_copies_keep_contents_after_the_outer_cell_ends() {
    for source in [
        "owner:1;copy:{holder:{->view:&owner};outer:&holder;->*outer};v:*(copy.view)",
        "owner:1;copy:{view:&owner;outer:&view;->*outer};v:*copy",
        "owner:1;copy:{view:&owner;outer:&view;second:&outer;->**second};v:*copy",
        "owner:1;result:{->view:&owner;outer:&view;->*outer};copy<&int32>:result;v:*copy",
        "owner:1;holder:{->view:&owner;->n:2};outer:&holder;view:&(outer.view);copy:*view;v:*copy",
        "owner:{->n:1};copy:{holder:{->view:&owner};outer:&holder;->&(outer.view.n)};v:*copy",
    ] {
        accepts(source);
    }
    for source in [
        "copy:{owner:1;holder:{->view:&owner};outer:&holder;->*outer}",
        "copy:{owner:1;view:&owner;outer:&view;->*outer}",
        "owner:1;copy:{holder:{->view:&owner};->&holder}",
        "owner:1;result:{->holder:{->view:&owner};->outer:&holder}",
        "d:@\"debug\";'done{result:'out{p:{owner:1;'out->holder:{->view:&owner;->n:2};->&holder};d.print(p.n);'done.leave()}}",
    ] {
        rejects(source, "E303");
    }
}

#[test]
pub(crate) fn transitive_projection_reads_only_selected_contents() {
    for source in [
        "owner:=1;holder:{->view:&owner;->n:2};outer:&holder;owner=3;v:outer.n",
        "owner:=1;holder:{->view:&owner;->n:2};outer:&holder;cell:&(outer.n);owner=3;v:*cell",
        "owner:=1;view:&owner;outer:&view;owner=3;same:outer==outer",
    ] {
        accepts(source);
    }
    for source in [
        "owner:=1;holder:{->view:&owner};outer:&holder;owner=3;copy:*outer;v:*(copy.view)",
        "owner:=1;view:&owner;owner=3;outer:&view;v:**outer",
        "owner:=1;holder:{->view:&owner;->n:2};outer:&holder;owner=3;v:*(outer.view)",
    ] {
        rejects(source, "E302");
    }
}

#[test]
pub(crate) fn transitive_summary_paths_stop_at_existing_work_limits() {
    let mut source = "owner:1;r0:&owner;".to_owned();
    for index in 1..256 {
        source.push_str(&format!("r{index}:&r{};", index - 1));
    }
    let errors = crate::compile(&source).unwrap_err();
    assert_eq!(errors[0].code, "B001", "{errors:?}");
    assert!(errors[0].message.contains("budget"));
    let mut aliases = "<R0>:<int32>;".to_owned();
    for index in 1..=64 {
        aliases.push_str(&format!("<R{index}>:<&R{}>;", index - 1));
    }
    accepts(&aliases);
    aliases.push_str("<R65>:<&R64>");
    rejects(&aliases, "B001");
}

#[test]
pub(crate) fn pointee_tags_keep_immutable_snapshots_and_mutable_read_freshness() {
    for source in [
        "owner:=1;holder:{->view<&int32><null>:null};outer:&holder;copy:*outer;owner=2;|copy.view<&int32>|{v:*(copy.view~<&int32>)}",
        "owner:=1;holder:{->view<&int32><null>:null};outer:&holder;cell:&(outer.view);copy:*cell;owner=2;|copy<&int32>|{v:*(copy~<&int32>)}",
    ] {
        accepts(source);
    }
    rejects(
        "owner:=1;tag<int32><null>:=null;tag=1;view:&tag;copy:*view;result<&int32><null>:{|copy<int32>|->&owner};owner=2;|result<&int32>|{v:*(result~<&int32>)}",
        "E302",
    );
}
