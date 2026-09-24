use super::{accepts, rejects};

#[test]
pub(crate) fn disjoint_emission_proofs_do_not_erase_result_loans() {
    rejects(
        "f<int32>:(flag<boolean>){a:=1;b:=2;p:=&a;value:'out{|flag|{p=&b;'out->p};|!flag|->p};a=3;b=3;->*value}",
        "E302",
    );
}

#[test]
pub(crate) fn guarded_assignments_keep_partial_baselines_and_complementary_overwrites() {
    for source in [
        "f<null>:(flag<boolean>){a:1;b:2;p:=&1;|flag|p=&a;|!flag|p=&b;value:*p}",
        "f<null>:(flag<boolean>){a:1;p:=&1;|flag|p=&a;|flag|value:*p}",
        "f<null>:(flag<boolean>){a:=1;b:=2;p:=&a;|flag|p=&b;|flag|{a=3;value:*p};|!flag|{b=3;value:*p}}",
    ] {
        accepts(source);
    }
    rejects(
        "f<null>:(flag<boolean>){a:1;p:=&1;|flag|p=&a;value:*p}",
        "E303",
    );
    rejects(
        "f<null>:(flag<boolean>){a:=1;b:2;p:=&a;|flag|p=&b;a=3;value:*p}",
        "E302",
    );
}

#[test]
pub(crate) fn nonreturning_arms_do_not_publish_reference_versions() {
    for source in [
        "d:@\"debug\";f<null>:(flag<boolean>){a:1;p:=&1;|flag|p=&a;|!flag|d.panic(\"stop\");value:*p}",
        "d:@\"debug\";f<null>:(flag<boolean>){a:1;b:=2;p:=&a;|flag|{p=&b;d.panic(\"stop\")};b=3;value:*p}",
        "d:@\"debug\";f<null>:(flag<boolean>){|flag|d.panic(\"stop\");a:1;p:=&a;p=&a;value:*p}",
        "d:@\"debug\";f<null>:(flag<boolean>,other<boolean>){a:1;p:=&a;|flag|{b:2;|other|p=&b;|other|d.panic(\"stop\")};value:*p}",
    ] {
        accepts(source);
    }
    rejects(
        "d:@\"debug\";f<null>:(flag<boolean>){a:=1;b:2;p:=&a;|flag|{p=&b;d.panic(\"stop\")};a=3;value:*p}",
        "E302",
    );
}

#[test]
pub(crate) fn short_circuit_updates_share_the_same_guarded_environment() {
    for source in [
        "f<null>:(flag<boolean>){a:1;b:2;p:=&1;x:flag&&{p=&a;->true};y:!flag&&{p=&b;->true};value:*p}",
        "f<null>:(flag<boolean>){a:1;p:=&1;x:flag||{p=&a;->false};|!flag|value:*p}",
        "a:1;b:2;p:=&a;skip:false&&{p=&b;->true};value:*p",
    ] {
        accepts(source);
    }
    rejects(
        "f<null>:(flag<boolean>){a:=1;b:2;p:=&a;skip:flag&&{p=&b;->true};a=3;value:*p}",
        "E302",
    );
    rejects(
        "d:@\"debug\";f<null>:(flag<boolean>){a:=1;b:2;p:=&a;skip:flag&&{p=&b;d.panic(\"stop\")};a=3;value:*p}",
        "E302",
    );
}

#[test]
pub(crate) fn guarded_pointer_versions_preserve_pointee_activity() {
    let prefix = "<H>:<{view<&int32><null>;n<int32>}>;";
    accepts(&format!(
        "{prefix}f<null>:(flag<boolean>){{a:=1;left<H>:{{->view:&a;->n:1}};right<H>:{{->n:2}};p:=&left;|flag|p=&right;copy:*p;|copy.view<null>|a=3;|copy.view<&int32>|value:*(copy.view~<&int32>)}}"
    ));
    rejects(
        &format!(
            "{prefix}f<null>:(flag<boolean>){{a:=1;left<H>:{{->view:&a;->n:1}};right<H>:{{->n:2}};p:=&left;|flag|p=&right;copy:*p;|copy.view<&int32>|{{a=3;value:*(copy.view~<&int32>)}}}}"
        ),
        "E302",
    );
}
