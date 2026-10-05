use super::{tests::checked, *};

mod dispatch;
mod limits;

#[test]
pub(crate) fn output_consumers_link_sparse_original_parts_through_local_copies() {
    let source = "d:@\"debug\";r:{->7;->tag:true};copy:((r));d.print(\"a{copy}b{1}c{((r))}d{({->9;->tag:false})}e\")";
    let (mut checker, reports) = checked(source);
    let (&id, _) = checker.outputs.first_key_value().unwrap();
    let Effect::Output(output) = &reports.effects[&id].1 else {
        panic!()
    };
    let expected = [1, 5, 7];
    assert_eq!(reports.slot_uses.len(), expected.len());
    for part in expected {
        let port = Port::Projection {
            point: id,
            step: part,
        };
        let (owner, slot) = reports.slot_uses[&port];
        assert_eq!(slot.index, 0);
        let input = output.parts[&part].input.unwrap();
        assert!(output.parts[&part].projection && input.primary);
        let anchor = checker
            .grouped_consumer(&reports, input.point, owner, Span::default())
            .unwrap()
            .unwrap();
        assert_eq!(reports.consumers[&anchor], (owner, slot.block));
    }
    assert!(
        !reports
            .slot_uses
            .contains_key(&Port::Projection { point: id, step: 3 })
    );
}

#[test]
pub(crate) fn output_consumers_keep_opaque_sources_and_prior_projections_before_stop() {
    for source in [
        "d:@\"debug\";f<null>:(r<{-><int32>;tag<boolean>}>){d.print(\"{r}\")}",
        "d:@\"debug\";r:={->7;->tag:true};d.print(\"{r}\")",
        "d:@\"debug\";f<{-><int32>;tag<boolean>}>:(){->7;->tag:true};d.print(\"{f()}\")",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty());
    }
    let source = "d:@\"debug\";r:{->7;->tag:true};d.print(\"a{r}b{d.panic(\"stop\")}tail\")";
    let (checker, reports) = checked(source);
    let (&id, _) = checker
        .outputs
        .iter()
        .find(|(_, output)| !output.panic)
        .unwrap();
    let Effect::Output(output) = &reports.effects[&id].1 else {
        panic!()
    };
    assert!(!output.terminal);
    assert!(output.stopped.is_some());
    assert_eq!(reports.slot_uses.len(), 1);
    assert!(
        reports
            .slot_uses
            .contains_key(&Port::Projection { point: id, step: 1 })
    );
}
