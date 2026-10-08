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
