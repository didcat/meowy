use super::super::capture::tests::capture;
use super::*;

#[test]
pub(crate) fn scalar_emission_values_preserve_tail_writes_without_publishing_the_block() {
    let source = "x:=7;{copy:x;->copy;x=9;x};x";
    let (mut checker, _, _, snapshots) = capture(source);
    let (_, op) = checker.emissions.first_key_value().unwrap();
    let id = op.targets[0].id;
    let target = op.targets[0].block;
    let start = source.find("x};").unwrap();
    let tail = *checker
        .local_reads
        .iter()
        .find(|(point, read)| {
            checker.points[**point].span == Span::new(start, start + 1) && read.owner == 0
        })
        .unwrap()
        .0;
    let value = snapshots.expressions[&tail].value.as_ref().unwrap();
    assert_eq!(
        value.domain,
        Domain::integer(9, &mut checker.flow, Span::default()).unwrap()
    );
    let emitted = snapshots.emissions[&id].value.as_ref().unwrap();
    assert_eq!(
        emitted,
        snapshots.expressions[&op.input].value.as_ref().unwrap()
    );
    assert_eq!(
        emitted.domain,
        Domain::integer(7, &mut checker.flow, Span::default()).unwrap()
    );
    assert_ne!(emitted.version, value.version);
    let Some(View::Known(before)) = snapshots.before(Port::Emission(id)) else {
        panic!()
    };
    let Some(View::Known(after)) = snapshots.after(Port::Emission(id)) else {
        panic!()
    };
    assert_eq!(before.cells, after.cells);
    assert!(matches!(
        snapshots.after(Port::BlockResult(target)),
        Some(View::Known(_))
    ));
    let parent = checker.bodies[&target].parent.unwrap();
    assert!(!snapshots.expressions.contains_key(&parent));
}

#[test]
pub(crate) fn scalar_emission_values_keep_stopped_tails_and_later_emissions_unavailable() {
    let (mut checker, _, _, snapshots) = capture("d:@\"debug\";v:{->7;d.panic(\"stop\")};{->9}");
    assert_eq!(snapshots.emissions.len(), 1);
    let (&id, saved) = snapshots.emissions.iter().next().unwrap();
    assert_eq!(
        saved.value.as_ref().unwrap().domain,
        Domain::integer(7, &mut checker.flow, Span::default()).unwrap()
    );
    let (statement, _) = checker.emission_sources[&id];
    let block = checker.emissions[&statement].targets[0].block;
    assert_eq!(snapshots.before(Port::BlockResult(block)), None);
    let (_, _, _, snapshots) = capture("'out{->{'out.leave()}}");
    assert!(snapshots.emissions.is_empty());
}

#[test]
pub(crate) fn scalar_emission_values_keep_unsupported_sources_and_compositions_conservative() {
    for source in ["x:1;{->&x}", "{->1.5}", "{->\"cat\"}"] {
        let (_, _, _, snapshots) = capture(source);
        assert_eq!(snapshots.emissions.len(), 1, "{source}");
        let (&id, fact) = snapshots.emissions.iter().next().unwrap();
        assert!(fact.value.is_none(), "{source}");
        assert_eq!(snapshots.after(Port::Emission(id)), Some(View::Missing));
    }
    let (_, _, _, snapshots) = capture("{->{->a:1}}");
    assert!(snapshots.emissions.is_empty());
}

#[test]
pub(crate) fn scalar_slot_emissions_keep_the_emitted_value_after_tail_writes() {
    let source = "x:=7;v:{->x;x=9};x";
    let (mut checker, _, _, snapshots) = capture(source);
    let op = checker.emissions.values().next().unwrap();
    let target = &op.targets[0];
    let Some(View::Known(before)) = snapshots.before(Port::Emission(target.id)) else {
        panic!()
    };
    assert_eq!(before.slot(target.block).unwrap().data, Data::Uninitialized);
    let Some(View::Known(after)) = snapshots.after(Port::BlockResult(target.block)) else {
        panic!()
    };
    let Data::Initialized {
        domain: Some(domain),
        version,
        origin,
    } = &after.slot(target.block).unwrap().data
    else {
        panic!()
    };
    assert_eq!(
        *domain,
        Domain::integer(7, &mut checker.flow, Span::default()).unwrap()
    );
    assert_eq!(*version, None);
    assert_eq!(
        *origin,
        snapshots.emissions[&target.id]
            .value
            .as_ref()
            .unwrap()
            .origin
    );
    let point = *checker.local_reads.keys().next_back().unwrap();
    assert_eq!(
        snapshots.expressions[&point].value.as_ref().unwrap().domain,
        Domain::integer(9, &mut checker.flow, Span::default()).unwrap()
    );
    assert!(
        !snapshots
            .expressions
            .contains_key(&checker.bodies[&target.block].parent.unwrap())
    );
}

#[test]
pub(crate) fn scalar_slot_emissions_preserve_nested_targets_and_function_sources() {
    for source in [
        "x:=7;v:'out{'inner{'out->x};x=9}",
        "f<int32>:(n<int32>)'out{'inner{'out->n}}",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let op = checker.emissions.values().next().unwrap();
        let target = &op.targets[0];
        let inner = checker.points[op.input].block.unwrap();
        assert_ne!(inner, target.block);
        let Some(View::Known(after)) = snapshots.after(Port::Emission(target.id)) else {
            panic!()
        };
        let Data::Initialized {
            domain: Some(domain),
            version,
            origin,
        } = &after.slot(target.block).unwrap().data
        else {
            panic!()
        };
        let value = snapshots.emissions[&target.id].value.as_ref().unwrap();
        assert_eq!(*domain, value.domain);
        assert_eq!(*origin, value.origin);
        assert_eq!(*version, None);
        assert_eq!(after.slot(inner).unwrap().data, Data::Uninitialized);
    }
}

#[test]
pub(crate) fn scalar_slot_emissions_keep_uncertain_initialization_without_overwriting() {
    for flag in ["true", "false"] {
        let source =
            format!("flag:={flag};v:'out{{|flag|'out->true;|flag|'out.leave();'out->false}}");
        let (checker, _, _, snapshots) = capture(&source);
        let Port::Leave(block) = checker.scope_exits.values().next().unwrap().edge.to else {
            panic!()
        };
        let Some(View::Known(after)) = snapshots.after(Port::BlockResult(block)) else {
            panic!()
        };
        let Data::Initialized {
            domain: Some(domain),
            ..
        } = &after.slot(block).unwrap().data
        else {
            panic!()
        };
        assert_eq!(*domain, Domain::boolean(flag == "true"));
    }
    let (checker, _, _, snapshots) =
        capture("f<boolean>:(flag<boolean>)'out{|flag|'out->true;|flag|'out.leave();'out->false}");
    let Port::Leave(block) = checker.scope_exits.values().next().unwrap().edge.to else {
        panic!()
    };
    assert!(snapshots.states.iter().any(|state| {
        state
            .slot(block)
            .is_some_and(|slot| slot.data == Data::Partial)
    }));
    assert_eq!(
        snapshots.after(Port::BlockResult(block)),
        Some(View::Missing)
    );
}

#[test]
pub(crate) fn scalar_slot_emissions_keep_incompatible_discarded_values_conservative() {
    use crate::check::dependencies::{ScalarKind, bodies::completion::Shape};
    let (checker, _, _, snapshots) =
        capture("d:@\"debug\";f:(flag<boolean>)'out{|flag|{'out->7;d.panic(\"stop\")}}");
    let op = checker.emissions.values().next().unwrap();
    let target = &op.targets[0];
    assert_eq!(
        checker.bodies[&target.block].completion.result,
        Shape::Scalar(ScalarKind::Null)
    );
    let Some(View::Known(before)) = snapshots.before(Port::Emission(target.id)) else {
        panic!()
    };
    assert_eq!(before.slot(target.block).unwrap().data, Data::Uninitialized);
    assert!(snapshots.emissions[&target.id].value.is_some());
    assert_eq!(
        snapshots.after(Port::Emission(target.id)),
        Some(View::Missing)
    );
}
