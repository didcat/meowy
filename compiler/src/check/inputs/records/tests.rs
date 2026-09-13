use super::Leaf;
use crate::check::inputs::tests::check;

#[test]
pub(crate) fn record_inputs_preserve_field_order_aliases_and_local_emission_dependencies() {
    let checker = check("row:{seed<uint8>:2;->z<uint8>:seed+1;->a<uint8>:{->z+1}};alias:row");
    assert_eq!(checker.record_inputs.len(), 2);
    for record in checker.record_inputs.values() {
        assert_eq!(
            record.values.values().copied().collect::<Vec<_>>(),
            [Leaf::Int(Some(4)), Leaf::Int(Some(3))]
        );
        assert!(record.input.error.is_none());
    }
}

#[test]
pub(crate) fn record_inputs_reject_effects_mutability_and_noninteger_shapes() {
    for source in [
        "d:@\"debug\";row:{->n:4;d.print(1)}",
        "row:{unused:=0;->n:4}",
        "row:={->n:4}",
        "row:{->n:=4}",
        "row:{->n:4;->text:\"x\"}",
        "row:{->4;->n:4}",
        "flag:=true;row:{|flag|->n:4}",
    ] {
        let checker = check(source);
        assert!(checker.record_inputs.is_empty(), "{source}");
    }
}

#[test]
pub(crate) fn record_inputs_retain_whole_initializer_errors_and_field_limits() {
    let checker =
        check("|false|{row<{good<uint8>;bad<uint8>}>:{->good<uint8>:4;->bad<uint8>:255+1}}");
    let record = checker.record_inputs.values().last().unwrap();
    assert_eq!(record.input.error.as_ref().unwrap().code, "E107");
    assert_eq!(
        record.values.values().copied().collect::<Vec<_>>(),
        [Leaf::Int(None), Leaf::Int(Some(4))]
    );
    let fields = (0..257).map(|id| format!("->n{id}:1;")).collect::<String>();
    assert!(check(&format!("row:{{{fields}}}")).record_inputs.is_empty());
}

#[test]
pub(crate) fn record_fields_copy_exact_values_without_losing_the_containing_evidence() {
    crate::compile("row:{->z<uint8>:3;->a<uint8>:4};copy:row.a;<T>:{n:copy;-><int32[n]>}").unwrap();
    let checker = check("row:{->z<uint8>:3;->a<uint8>:4};copy:row.a;alias:row;next:alias.z");
    let values = checker
        .inputs
        .values()
        .filter_map(|input| input.value)
        .collect::<Vec<_>>();
    assert_eq!(values, [4, 3]);
    let source = "|false|{row<{good<uint8>;bad<uint8>}>:{->good<uint8>:4;->bad<uint8>:255+1};copy:row.good;<T>:{n:copy;-><int32>}}";
    let error = crate::compile(source).unwrap_err().remove(0);
    assert_eq!(error.code, "E107");
    assert_eq!(error.span.start, source.find("255+1").unwrap());
}

#[test]
pub(crate) fn record_fields_copies_cannot_hide_effects_or_mutable_siblings() {
    for source in [
        "d:@\"debug\";row:{->good:4;d.print(1)};copy:row.good;<T>:{n:copy;-><int32>}",
        "row:{->good:4;->mutable:=0};copy:row.good;<T>:{n:copy;-><int32>}",
        "row:={->good:4};copy:row.good;<T>:{n:copy;-><int32>}",
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            "E211",
            "{source}"
        );
    }
}

#[test]
pub(crate) fn record_fields_feed_required_scratch_extents_and_function_scopes() {
    for source in [
        "row:{->width<uint8>:4};<T>:{n:row.width;-><int32[n]>};v<T>:[1,2]",
        "row:{->width<uint8>:4};alias:row;<T>:{-><int32[(alias).width]>}",
        "row:{->width:4};f<int32>:(){<T>:{n:row.width;-><int32[n]>};v<T>:[1,2];->v[2]}",
        "core:@\"core\";<T>:{element:core.int32;->element}",
    ] {
        crate::compile(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
    }
    let source = "row:{->width<uint8>:255};<T>:{n:row.width+1;-><int32>}";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E107");
    let source = "row:{->width:4};<T>:{n:row.missing;-><int32>}";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E201");
}

#[test]
pub(crate) fn record_fields_require_whole_record_purity_and_retain_error_spans() {
    for source in [
        "d:@\"debug\";row:{->good:4;d.print(1)};<T>:{n:row.good;-><int32>}",
        "row:{->good:4;->mutable:=0};<T>:{n:row.good;-><int32>}",
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            "E211",
            "{source}"
        );
    }
    let source = "|false|{row<{good<uint8>;bad<uint8>}>:{->good<uint8>:4;->bad<uint8>:255+1};<T>:{n:row.good;-><int32>}}";
    let error = crate::compile(source).unwrap_err().remove(0);
    assert_eq!(error.code, "E107");
    assert_eq!(error.span.start, source.find("255+1").unwrap());
    assert_eq!(
        crate::compile("row:{->width:4};f<int32>:(){->row.width}").unwrap_err()[0].code,
        "B001"
    );
}

#[test]
pub(crate) fn nested_record_evidence_keeps_paths_and_scoped_record_aliases() {
    let checker = check("row:{->z:{->width<uint8>:3};copy:z;->a:copy}");
    let record = checker.record_inputs.values().last().unwrap();
    assert_eq!(record.values.get(&vec![0, 0]), Some(&Leaf::Int(Some(3))));
    assert_eq!(record.values.get(&vec![1, 0]), Some(&Leaf::Int(Some(3))));
    assert!(record.input.error.is_none());
}

#[test]
pub(crate) fn nested_record_evidence_rejects_effects_and_mutation_in_any_descendant() {
    for source in [
        "d:@\"debug\";row:{->a:{->n:4};->b:{->n:1;d.print(1)}}",
        "row:{->a:{->n:4};->b:{->n:=1}}",
        "row:{->a:{->n:4};->b:{->text:\"x\"}}",
    ] {
        assert!(check(source).record_inputs.is_empty(), "{source}");
    }
    let checker = check(
        "|false|{part<{good<uint8>;bad<uint8>}>:{->good<uint8>:4;->bad<uint8>:255+1};row<{nested<{good<uint8>;bad<uint8>}>}>:{->nested:part}}",
    );
    assert_eq!(
        checker
            .record_inputs
            .values()
            .last()
            .unwrap()
            .input
            .error
            .as_ref()
            .unwrap()
            .code,
        "E107"
    );
}

#[test]
pub(crate) fn nested_record_evidence_bounds_total_fields_and_record_depth() {
    use crate::hir::{Field, Type};
    let mut ty = Type::Int {
        bits: 32,
        signed: true,
    };
    let mut checker = crate::check::Checker::new();
    for _ in 0..32 {
        ty = Type::Record {
            primary: Box::new(Type::Null),
            fields: vec![Field {
                name: "n".into(),
                ty,
                mutable: false,
            }],
        };
    }
    assert!(checker.record_shape(&ty));
    ty = Type::Record {
        primary: Box::new(Type::Null),
        fields: vec![Field {
            name: "n".into(),
            ty,
            mutable: false,
        }],
    };
    assert!(!checker.record_shape(&ty));
    let fields = (0..256).map(|id| format!("->n{id}:1;")).collect::<String>();
    assert!(
        check(&format!("row:{{->nested:{{{fields}}}}}"))
            .record_inputs
            .is_empty()
    );
}

#[test]
pub(crate) fn nested_record_evidence_checks_unused_local_shapes() {
    let fields = (0..257).map(|id| format!("->n{id}:1;")).collect::<String>();
    assert!(
        check(&format!("row:{{unused:{{{fields}}};->n:1}}"))
            .record_inputs
            .is_empty()
    );
}

#[test]
pub(crate) fn nested_record_paths_support_direct_copied_and_grouped_fields() {
    for source in [
        "row:{->nested:{->width<uint8>:4}};<T>:{n:row.nested.width;-><int32[n]>}",
        "row:{->nested:{->width<uint8>:4}};alias:row.nested;copy:alias.width;<T>:{n:copy;-><int32[n]>}",
        "row:{->nested:{->width<uint8>:4}};<T>:{-><int32[((row).nested).width]>}",
        "row:{->nested:{->width<uint8>:4}};f<int32>:(){<T>:{n:row.nested.width;-><int32[n]>};v<T>:[1,2];->v[2]}",
        "row:{->nested:{->width<uint8>:4};->copy:nested.width};<T>:{n:row.copy;-><int32[n]>}",
    ] {
        crate::compile(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
    }
}

#[test]
pub(crate) fn nested_record_paths_keep_complete_ancestor_errors_through_aliases() {
    let source = "|false|{part<{good<uint8>;bad<uint8>}>:{->good<uint8>:4;->bad<uint8>:255+1};row<{nested<{good<uint8>;bad<uint8>}>}>:{->nested:part};alias:row.nested;copy:alias.good;<T>:{n:copy;-><int32>}}";
    let error = crate::compile(source).unwrap_err().remove(0);
    assert_eq!(error.code, "E107");
    assert_eq!(error.span.start, source.find("255+1").unwrap());
    for source in [
        "d:@\"debug\";row:{->good:{->n:4};d.print(1)};alias:row.good;<T>:{n:alias.n;-><int32>}",
        "row:{->good:{->n:4};->bad:{->n:=1}};alias:row.good;<T>:{n:alias.n;-><int32>}",
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            "E211",
            "{source}"
        );
    }
}

#[test]
pub(crate) fn nested_record_paths_keep_missing_fields_references_and_captures_gated() {
    for (source, code) in [
        (
            "row:{->nested:{->n:4}};<T>:{n:row.nested.missing;-><int32>}",
            "E201",
        ),
        (
            "row:{->nested:{->n:4}};p:&row;<T>:{n:p.nested.n;-><int32>}",
            "B001",
        ),
        ("row:{->nested:{->n:4}};f<int32>:(){->row.nested.n}", "B001"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn record_boolean_leaves_keep_their_kind_and_ancestor_errors() {
    let checker = check("row:{->flag:true;->width<uint8>:4}");
    let record = checker.record_inputs.values().last().unwrap();
    assert_eq!(record.values[&vec![0]], Leaf::Bool(Some(true)));
    assert!(record.field(&[0]).is_none());
    assert_eq!(record.field(&[1]).unwrap().value, Some(4));
    let checker = check(
        "|false|{row<{flag<boolean>;width<uint8>}>:{base<uint8>:255;bad:base+1==0;->flag:true;->width<uint8>:4}}",
    );
    let record = checker.record_inputs.values().last().unwrap();
    assert_eq!(record.values[&vec![0]], Leaf::Bool(None));
    assert_eq!(record.values[&vec![1]], Leaf::Int(None));
    assert!(record.field(&[0]).is_none());
    assert_eq!(record.field(&[1]).unwrap().error.unwrap().code, "E107");
    for count in [256, 257] {
        let fields = (0..count)
            .map(|id| format!("->f{id}:true;"))
            .collect::<String>();
        assert_eq!(
            check(&format!("row:{{{fields}}}")).record_inputs.is_empty(),
            count == 257
        );
    }
}
