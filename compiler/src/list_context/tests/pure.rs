use super::rejects;

#[test]
pub(crate) fn bit_call_candidates_retain_lexical_identity_and_contextual_widths() {
    for source in [
        "b:@\"bits\";values<int8[1]><uint8[1]>:[b.not(128)]",
        "b:@\"bits\";flip:b.not;values<int8[1]><uint8[1]>:[128.(flip)]",
        "values<int8[1]><uint8[1]>:[(@\"bits\").not(128)]",
        "b:@\"bits\";alias:b;values<int8[1]><uint8[1]>:[alias.and(255,128)]",
        "b:@\"bits\";values<int8[2]><uint8[2]>:[b.or(128,0),{->b.xor(1,1)}]",
    ] {
        crate::compile(source).unwrap();
    }
    rejects("b:@\"bits\";values<int8[1]><uint8[1]>:[b.not(1)]", "E207");
    rejects(
        "b:@\"bits\";values<int8[1]><uint8[1]>:[b.and(256,0)]",
        "E207",
    );
    rejects("b:@\"bits\";values<int8[1]><uint8[1]>:[b.and(128)]", "E212");
    rejects(
        "b:@\"bits\";{b:{->not:1};values<int8[1]><uint8[1]>:[b.not(128)]}",
        "B001",
    );
}

#[test]
pub(crate) fn pure_compounds_reuse_contextual_operator_rules() {
    for source in [
        "values<int8[1]><int16[1]>:[-(128)]",
        "values<int8[1]><int16[1]>:[-(-128)]",
        "values<int8[1]><int16[1]>:[(127+1)-1]",
        "values<int8[1]><uint8[1]>:[~128]",
        "values<float32[1]><float64[1]>:[1e39-1e39]",
        "values<boolean[1]><int32[1]>:[!(false||true)&&false]",
        "values<boolean[1]><int32[1]>:[false&&(1/0==1)]",
        "values<boolean[1]><int32[1]>:[true||(1/0==1)]",
        "values<int8[1]><string[1]>:[(-128)%-1]",
        "values<int8[1][1]><int16[1][1]>:[[(127+1)-1]]",
        "<A>:<{value<int8>}>;<B>:<{value<int16>}>;values<A[1]><B[1]>:[{->value:127+1}]",
    ] {
        assert!(crate::compile(source).is_ok(), "{source}");
    }
    for source in [
        "values<int8[1]><uint8[1]>:[~1]",
        "values<float32[1]><float64[1]>:[3e38+3e38]",
        "values<int8[1]><uint8[1]>:[256-256]",
        "values<int8[1]><int16[1]>:[1/0]",
    ] {
        rejects(source, "E207");
    }
    rejects("values<int8[2]><string[1]>:[127+1,0]", "E107");
    rejects("values<boolean[1]><string[1]>:[true&&(1/0==1)]", "E107");
}

#[test]
pub(crate) fn pure_constant_leaves_retain_types_and_lexical_identity() {
    for source in [
        "byte<uint8>:254;values<uint8[1]><uint16[1]>:[byte+1]",
        "flag:false;values<boolean[1]><int32[1]>:[!flag]",
        "f<null>:(){true:false;values<boolean[1]><int32[1]>:[true&&(1/0==0)]}",
    ] {
        assert!(crate::compile(source).is_ok(), "{source}");
    }
    rejects(
        "byte<uint8>:255;values<uint8[1]><uint16[1]>:[byte+1]",
        "E107",
    );
    rejects(
        "byte<uint8>:1;values<uint16[1]><string[1]>:[byte+1]",
        "E207",
    );
    rejects("byte<uint8>:1;values:[byte,1+1]", "E207");
    rejects(
        "byte<uint8>:1;f<null>:(){values<uint8[1]><uint16[1]>:[byte+1]}",
        "B001",
    );
}

#[test]
pub(crate) fn compound_candidates_use_their_original_evaluation_reach() {
    for source in [
        "d:@\"debug\";values<int8[2]><uint8[2]>:[127+1,{d.print(1);->1}]",
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};values<int8[2]><uint8[2]>:[127+1,stop()]",
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};values<int8[2]><string[2]>:[stop(),127+1]",
    ] {
        assert!(crate::compile(source).is_ok(), "{source}");
    }
    rejects(
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};values<int8[2]><uint8[2]>:[stop(),127+1]",
        "E207",
    );
    rejects(
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};values<int8[2]><uint8[2]>:[stop(),256-256]",
        "E207",
    );
    rejects(
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};values<int8[2]><string[2]>:[127+1,stop()]",
        "E107",
    );
}

#[test]
pub(crate) fn scalar_probe_work_includes_referenced_constant_bytes() {
    let source = format!(
        "text:\"{}\";values<boolean[1]><string[1]>:[text==text]",
        "a".repeat(500_000)
    );
    rejects(&source, "B001");
    let expression = (0..64).map(|_| "1").collect::<Vec<_>>().join("+");
    let types = (1..=64)
        .map(|capacity| format!("<int32[{capacity}]>"))
        .collect::<String>();
    rejects(&format!("values{types}:[{expression}]"), "B001");
    let values = vec!["1"; super::MAX_SCALAR_NODES].join(",");
    rejects(
        &format!(
            "d:@\"debug\";values<int8[4096][1]><int16[4096][1]>:[{{d.print(1);->[{values}]}}]"
        ),
        "B001",
    );
}
