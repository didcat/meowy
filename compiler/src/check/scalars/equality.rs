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

#[test]
pub(crate) fn shared_operand_hints_allow_record_fields_and_nested_composition() {
    for op in ["==", "!="] {
        for body in [
            "{->p;->tag:true}",
            "((({->p;->tag:true})))",
            "p.{->$;->tag:true}",
            "((p.{->$;->tag:true}))",
            "{->(({->p;->first:true}));->tag:300}",
            "'outer{{'outer->p;'outer->tag:true}}",
        ] {
            accepts(&format!("n:1;p:&n;x:p{op}{body};y:{body}{op}p"));
        }
    }
    for source in [
        "n:1;p:&n;r:{->p;->first:true};x:p=={->r;->tag:false}",
        "f<boolean>:(p<&int32>){->p==0.{->p;->tag:true}};n:1;x:f(&n)",
    ] {
        accepts(source);
    }
}

#[test]
pub(crate) fn shared_operand_hints_preserve_type_lifetime_and_loan_errors() {
    for source in [
        "n:1;p:&n;x:p=={->&true;->tag:true}",
        "n<uint8>:1;p:&n;x:p==(&1).{->$;->tag:true}",
        "n:1;p:&n;x:p=={->p;->tag<int32>:{->1;->extra:true}}",
    ] {
        rejects(source, "E207");
    }
    rejects("n:=1;p:&n;x:p=={->&!n;->tag:true}", "E302");
    rejects("n:1;p:&n;x:p=={m:1;->&m;->tag:true}", "E303");
    rejects("n:1;p:&n;flag:=false;x:p=={|flag|->p;->tag:true}", "E204");
    rejects("n:1;p:&n;x:p=={->p;->tag:true;->tag:false}", "E205");
    rejects("n:=1;p:&n;x:p==((&!n).{->&n;->tag:true})", "B001");
    for source in [
        "n:=1;m:2;p:&m;r:&!n;x:p=={->r;->tag:true};*r=3",
        "n:=1;m:2;p:&m;r:&!n;x:(({->r;->tag:true}))==p;*r=3",
        "n:=1;m:2;p:&m;r:&!n;q<&int32>:r;row:{->q;->tag:true};x:p==row;*r=3",
    ] {
        let errors = crate::compile(source).unwrap_err();
        assert_eq!(errors[0].code, "B001");
        assert!(
            errors[0]
                .message
                .contains("exclusive ancestry crossing a call, result or reference cell")
        );
    }
}
