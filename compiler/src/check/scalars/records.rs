use super::super::tests::{accepts, rejects};

#[test]
pub(crate) fn operand_record_contexts_keep_explicit_result_and_field_constraints() {
    for source in [
        "x<int32>:{->3;->tag:true}",
        "f<int32>:(){->3;->tag:true}",
        "x<int32>:3.{->$;->tag:true}",
        "x<{tag<boolean>}>:{->tag:true;->extra:1}",
        "x<{-><int32>;tag<boolean>}>:{->3;->tag:1}",
        "a:{->3;->tag:true};x:a=={->3;->tag:true;->extra:1}",
    ] {
        rejects(source, "E207");
    }
    rejects("f<int32>:(){flag:=false;|flag|->1}", "E204");
    accepts("a:{->3;->tag:true};x:a=={->3;->tag:false}");
    accepts("a:3.{->$;->tag:true};x:a+(4.{->$;->tag:false}~<{-><int32>;tag<boolean>}>)");
}

#[test]
pub(crate) fn operand_record_contexts_keep_scalar_widths_and_initialization_checks() {
    for source in [
        "n<uint8>:7;x:n+{->7};y:{->7}+n",
        "n<uint64>:7;x:n+(({->4294967296}))",
        "n<float32>:1.5;x:(({->2.0}))+n",
    ] {
        accepts(source);
    }
    for source in ["n<uint8>:7;x:n+{->256}", "n<uint8>:7;x:(({->256}))+n"] {
        rejects(source, "E216");
    }
    rejects("n<uint8>:7;wide<uint16>:1;x:n+{->wide}", "E207");
    rejects("n<uint8>:7;flag:=false;x:n+{|flag|->1}", "E204");
}

#[test]
pub(crate) fn operand_record_hints_allow_fields_in_blocks_groups_and_dispatches() {
    for op in ["+", "-", "*", "/", "%", "<", "<=", ">", ">="] {
        for body in [
            "{->4;->tag:false}",
            "((({->4;->tag:false})))",
            "4.{->$;->tag:false}",
            "((4.{->$;->tag:false}))",
        ] {
            accepts(&format!(
                "a:3.{{->$;->tag:true}};x:a{op}{body};y:{body}{op}a"
            ));
        }
    }
    accepts("x:3=={->3;->tag:true};y:false!={->true;->tag:true}");
    accepts("x:\"cat\"<{->\"dog\";->tag:true};y:null=={->tag:true}");
}

#[test]
pub(crate) fn operand_record_hints_keep_primary_widths_and_nested_composition() {
    for source in [
        "n<uint8>:7;x:n+{->255;->tag:300};y:(({->255;->tag:300}))+n",
        "n<uint8>:7;x:n+0.{->1;->tag:true}",
        "n<uint64>:7;x:n+{->{->4294967296;->a:true};->z:300}",
        "n<uint64>:7;x:n+{->(({->4294967296;->a:true}));->z:300}",
        "n<float32>:1.5;x:(({->2.0;->tag:true}))+n",
        "r:{->3;->a:true};x:1+{->r;->z:false}",
        "x:1+'outer{{'outer->2;'outer->tag:true}}",
        "flag:=true;x:1+'outer{|flag|{'outer->2;'outer->tag:true;'outer.leave()};->3;->tag:false}",
    ] {
        accepts(source);
    }
    for source in [
        "n<uint8>:7;x:n+{->256;->tag:true}",
        "n<uint8>:7;x:n+0.{->256;->tag:true}",
    ] {
        rejects(source, "E216");
    }
    rejects("n<uint8>:7;x:n+0.{->$;->tag:true}", "E207");
    rejects("flag:=false;x:1+{|flag|->3;->tag:true}", "E204");
    rejects("x:1+{->3;->tag:true;->tag:false}", "E205");
    rejects("x:1+{->(({->3;->tag:true}));->tag:false}", "E205");
    rejects("x:1+{->3;->tag<int32>:{->4;->extra:true}}", "E207");
}
