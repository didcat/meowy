use super::{Checker, tests::check};

pub(crate) fn marked(source: &str) -> Checker {
    let mut checker = Checker::new();
    extend(&mut checker, source);
    for input in checker.inputs.values_mut() {
        input.derived = true;
    }
    for input in checker.bool_inputs.values_mut() {
        input.derived = true;
    }
    checker
}

pub(crate) fn extend(checker: &mut Checker, source: &str) {
    let block = crate::parser::parse(source).unwrap();
    for stmt in &block.stmts {
        checker.stmt(stmt).unwrap();
    }
}

#[test]
pub(crate) fn proof_evidence_survives_scalar_copies_and_arithmetic() {
    let mut checker = marked("seed<uint8>:3");
    extend(&mut checker, "copy:seed;sum:copy+1;flag:sum==4;again:!flag");
    assert!(checker.inputs.values().all(|input| input.derived));
    assert!(checker.bool_inputs.values().all(|input| input.derived));
    assert_eq!(checker.inputs[&2].value, Some(4));
    assert_eq!(checker.bool_inputs[&4].value, Some(false));
}

#[test]
pub(crate) fn proof_evidence_survives_selected_block_conditions_and_tails() {
    for flag in ["true", "false"] {
        for source in [
            "n:{|flag|unused:3;->4};copy:n",
            "b:{|flag|unused:true;->false};copy:b",
            "n:{->3;unused:flag};copy:n",
        ] {
            let mut checker = marked(&format!("flag:{flag}"));
            extend(&mut checker, source);
            let marks = if source.starts_with("n:") {
                checker
                    .inputs
                    .values()
                    .rev()
                    .take(2)
                    .map(|input| input.derived)
                    .collect::<Vec<_>>()
            } else {
                checker
                    .bool_inputs
                    .values()
                    .rev()
                    .take(2)
                    .map(|input| input.derived)
                    .collect::<Vec<_>>()
            };
            assert_eq!(marks, [true, true], "{source}");
        }
    }
}

#[test]
pub(crate) fn proof_evidence_survives_nested_record_copies_and_leaf_kinds() {
    let mut checker = marked("seed:3;flag:true");
    extend(
        &mut checker,
        "row:{->inner:{->n:seed;->b:flag}};copy:row;part:copy.inner;n:part.n;b:part.b",
    );
    assert_eq!(checker.record_inputs.len(), 4);
    assert!(checker.record_inputs.values().all(|row| row.input.derived));
    assert!(checker.inputs.values().all(|input| input.derived));
    assert!(checker.bool_inputs.values().all(|input| input.derived));
}

#[test]
pub(crate) fn proof_evidence_keeps_failures_and_unrelated_inputs_separate() {
    let mut checker = marked("seed<uint8>:255");
    extend(
        &mut checker,
        "|false|{bad:seed+1;copy:bad};plain:7;flag:false",
    );
    for id in [1, 2] {
        let input = &checker.inputs[&id];
        assert!(input.derived);
        assert_eq!(input.error.as_ref().unwrap().code, "E107");
        assert_eq!(input.value, None);
    }
    assert_eq!(
        checker.inputs[&1].error.as_ref().unwrap().span,
        checker.inputs[&2].error.as_ref().unwrap().span,
    );
    assert!(!checker.inputs[&3].derived);
    assert!(!checker.bool_inputs[&4].derived);
}

#[test]
pub(crate) fn ordinary_evidence_and_fixed_proof_metadata_are_not_derived() {
    let checker = check(
        "p:@\"proof\";r:p.can_copy<uint32>();<Flag>:r.always<>;flag<Flag>:true;revision:p.revision;copy:revision;row:{->b:flag;->n:copy}",
    );
    assert!(checker.inputs.values().all(|input| !input.derived));
    assert!(checker.bool_inputs.values().all(|input| !input.derived));
    assert!(checker.record_inputs.values().all(|row| !row.input.derived));
}
