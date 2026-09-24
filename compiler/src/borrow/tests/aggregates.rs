use super::{accepts, rejects};

#[test]
pub(crate) fn record_origins_preserve_primary_named_and_nested_components() {
    accepts(
        "a:1;b:2;r<{left<&int32>;right<&int32>}>:{->left:&a;->right:&b};copy:r;x:*(copy.left);y:*(copy.right)",
    );
    accepts("a:1;b:2;r:{->&a;->ref:&b};copy:{->r};view<&int32>:copy;x:*view;y:*(copy.ref)");
    accepts("a:1;r:{->outer:{->inner:&a}};copy:r.outer;view:copy.inner;x:*view");
    accepts("a:1;r:{b:2;pair:{->safe:&a;->bad:&b};copy:pair;->copy.safe};x:*r");
    rejects("a:1;r:{b:2;pair:{->safe:&a;->bad:&b};->pair}", "E303");
    rejects("r:{a:1;->outer:{->inner:&a}}", "E303");
}

#[test]
pub(crate) fn each_record_component_retains_its_own_guarded_origins() {
    accepts(
        "f<int32>:(flag<boolean>){a:1;b:2;r:{|flag|->left:&a;|!flag|->left:&b;->right:&a};->*(r.left)+*(r.right)}",
    );
    rejects(
        "a:1;flag:=true;r:'out {b:2;|flag|'out->field:&a;|!flag|'out->field:&b}",
        "E303",
    );
    accepts(
        "a:1;again:=true;r:'out {|again|{b:2;'out->field:&b;again=false;'out.restart()};->field:&a};x:*(r.field)",
    );
    accepts(
        "a:1;d:@\"debug\";f<int32>:(flag<boolean>){b:2;r:'out {|flag|{c:3;'out->nested:{->view:&c};d.panic(\"stop\")};->nested:{->view:&b}};->*(r.nested.view)}",
    );
}

#[test]
pub(crate) fn nullable_references_preserve_active_and_absent_origins() {
    accepts("a:1;u<&int32><null>:&a;copy:u;|copy<&int32>|{r<&int32>:copy;x:*r}");
    accepts("u<&int32><null>:null;|u<&int32>|{r<&int32>:u;x:*r}");
    accepts("a:1;flag:=true;u<&int32><null>:{|flag|->&a};|u<&int32>|x:*(u~<&int32>)");
    accepts(
        "a:1;r<{view<&int32><null>;count<int32>}>:{->count:1};|r.view<&int32>|x:*(r.view~<&int32>)",
    );
    rejects("u<&int32><null>:{a:1;->&a}", "E303");
    rejects("flag:=true;u:{a:1;|flag|->view:&a}", "E303");
}

#[test]
pub(crate) fn retagging_maps_reference_origins_by_member_type() {
    accepts(
        "a:1;u<&int32><null>:&a;wide<boolean><&int32><null>:u;|wide<&int32>|{r<&int32>:wide;x:*r}",
    );
    accepts(
        "a:1;b<int64>:2;flag:=true;u<&int32><&int64><null>:{|flag|->&a;|!flag|->&b};|u<&int32>|x:*(u~<&int32>);|u<&int64>|y:*(u~<&int64>)",
    );
    accepts(
        "a:1;flag:=true;u<&int32><null>:{inner<&int32><null>:{|flag|->&a};|inner<&int32>|->inner~<&int32>};copy:u;|copy<&int32>|x:*(copy~<&int32>)",
    );
}

#[test]
pub(crate) fn nested_union_fields_preserve_variant_specific_activity() {
    accepts(
        "<A>:<{kind<boolean>;view<&int32><null>}>;<B>:<{kind<int32>;view<&int32><null>}>;f<null>:(flag<boolean>){a:1;left<A>:{->kind:true;->view:&a};right<B>:{->kind:2};u<A><B>:{|flag|->left;|!flag|->right};|u<A>|{|u.view<&int32>|x:*(u.view~<&int32>)};|u<B>|{|u.view<null>|{}}}",
    );
    rejects(
        "<A>:<{view<&int32>}>;u<A><null>:{a:1;r<A>:{->view:&a};->r}",
        "E303",
    );
    accepts(
        "a:1;again:=true;u<{view<&int32><null>}>:'out {|again|{b:2;'out->view:&b;again=false;'out.restart()}};|u.view<&int32>|x:*(u.view~<&int32>)",
    );
}
