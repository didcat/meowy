use super::super::tests::{accepts, rejects};

#[test]
pub(crate) fn equality_contexts_keep_shared_conversions_and_declared_constraints() {
    for source in [
        "n:1;p:&n;r:{->p;->tag:true};x:p==r;y:r==p",
        "n:1;p:&n;x:p==({->p;->tag:true}~<{-><&int32>;tag<boolean>}>)",
        "n:1;p:&n;x:p==((p.{->$;->tag:true}~<{-><&int32>;tag<boolean>}>))",
        "n:=1;m:2;p:&m;r:&!n;x:p=={->r};*r=3",
        "n:1;p:&n;x:p==(({->p}));y:((p.{->$}))==p",
    ] {
        accepts(source);
    }
    for source in [
        "n:1;p:&n;x<&int32>:{->p;->tag:true}",
        "n:1;p:&n;x<{-><&int32>}>:{->p;->tag:true}",
        "n:1;p:&n;x:p=={->null}",
        "n:1;p:&n;x:p=={->&true}",
    ] {
        rejects(source, "E207");
    }
    rejects("n:=1;p:&n;x:p=={->&!n}", "E302");
    rejects("n:1;p:&n;x:p=={m:1;->&m}", "E303");
    rejects("n:=1;p:&n;x:p==((&!n).{->&n;->tag:true})", "B001");
}

#[test]
pub(crate) fn equality_contexts_keep_list_capacity_elements_and_full_records() {
    for source in [
        "a<uint8[3]>:[1];x:a=={->[1,2]};y:(({->[1,2]}))==a",
        "a<uint8[3]>:[1];x:a==0.{->[2]};y:((0.{->[1]}))==a",
        "a<int32[3]>:[1];x:a==(([1,2]));y:a!=[]",
        "a:{->[1];->tag:true};x:a=={->[1];->tag:false}",
        "a:{->[1];->tag:true};x:a==(([1].{->$;->tag:true}))",
    ] {
        accepts(source);
    }
    rejects("a<uint8[3]>:[1];x:a=={->[256]}", "E216");
    rejects("a<uint8[1]>:[1];x:a=={->[1,2]}", "E103");
    rejects("a<uint8[2]>:[1];b<uint16[2]>:[1];x:a==b", "E222");
    rejects(
        "a:{->[1];->tag:true};x:a=={->[1];->tag:true;->extra:1}",
        "E207",
    );
}
