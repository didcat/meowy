use super::{super::tests::checked, *};
use crate::check::dependencies::{edges::forward::effects::Effect, emissions::Projection};

#[test]
pub(crate) fn result_source_index_keeps_composed_projections_and_owner_boundaries() {
    let source =
        "source:{->7;->a:1;->b:true};copy:'out{{'out->source}};f<int32>:(){->4};row:{->xs:[1,2]}";
    let (mut checker, reports) = checked(source);
    let mut parts = MAX_EDGES;
    let index = checker
        .result_source_index(&reports, &mut parts, Span::default())
        .unwrap();
    let (&statement, op) = checker
        .emissions
        .iter()
        .find(|(_, op)| op.targets.len() == 3)
        .unwrap();
    let block = op.targets[0].block;
    assert_ne!(checker.points[statement].block, Some(block));
    for (target, projection) in [
        Projection::Primary,
        Projection::Field(0),
        Projection::Field(1),
    ]
    .into_iter()
    .enumerate()
    {
        let value = &op.targets[target];
        let row = &index[&(block, value.field.as_deref())];
        assert_eq!(value.projection, projection);
        assert!(!row.mutable);
        assert_eq!(
            row.values,
            [Candidate {
                emission: value.id,
                statement,
                target,
            }]
        );
    }
    for (&owner, &(block, _)) in &reports.entries {
        assert_eq!(reports.results[&block].0, owner);
        assert!(reports.results[&block].1.consumer.is_none());
    }
    assert!(reports.results[&block].1.consumer.is_some());
    assert!(reports.results.values().any(|(_, result)| {
        result
            .slots
            .as_ref()
            .is_some_and(|slots| slots.contains(&Sources::Unknown))
    }));
}

#[test]
pub(crate) fn result_source_index_requires_independent_target_and_block_visits() {
    let (mut checker, mut reports) = checked("source:{->7;->a:1;->b:true};copy:{->source}");
    let (&statement, op) = checker
        .emissions
        .iter()
        .find(|(_, op)| op.targets.len() == 3)
        .unwrap();
    let block = op.targets[0].block;
    for (_, effect) in reports.effects.values_mut() {
        if let Effect::Emission(op) = effect {
            op.initialized.fill(false);
            op.result = true;
        }
    }
    let mut parts = 2;
    assert!(
        checker
            .result_source_index(&reports, &mut parts, Span::default())
            .unwrap()
            .is_empty()
    );
    assert_eq!(parts, 2);
    let (_, Effect::Emission(op)) = reports.effects.get_mut(&statement).unwrap() else {
        panic!()
    };
    op.initialized[1] = true;
    op.result = false;
    let index = checker
        .result_source_index(&reports, &mut parts, Span::default())
        .unwrap();
    assert_eq!(parts, 0);
    assert_eq!(index.len(), 1);
    assert_eq!(index[&(block, Some("a"))].values[0].target, 1);
    for remove in [false, true] {
        if remove {
            reports.blocks.remove(&block);
        } else {
            reports.blocks.get_mut(&block).unwrap().1.result = false;
        }
        assert!(
            checker
                .result_source_index(&reports, &mut parts, Span::default())
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
pub(crate) fn result_source_index_preserves_discarded_fields_primaries_and_mutable_aliases() {
    let source = "d:@\"debug\";flag:=false;row:'out{|flag|{'out->gone:9;'out->x:=1;x=2;d.panic(\"stop\")};->x:3};value:'out{|flag|{'out->7;d.panic(\"stop\")}}";
    let (mut checker, reports) = checked(source);
    let mut parts = MAX_EDGES;
    let index = checker
        .result_source_index(&reports, &mut parts, Span::default())
        .unwrap();
    let (&(block, _), row) = index
        .iter()
        .find(|((_, name), _)| *name == Some("x"))
        .unwrap();
    assert!(row.mutable);
    assert_eq!(row.values.len(), 2);
    assert!(index.contains_key(&(block, Some("gone"))));
    let Layout::Slots(layout) = &checker.bodies[&block].layout else {
        panic!()
    };
    assert_eq!(layout.len(), 2);
    assert!(!layout[1].mutable);
    assert_eq!(layout[1].field.as_deref(), Some("x"));
    assert_eq!(
        reports.results[&block].1.slots.as_ref().unwrap()[1],
        Sources::Unknown
    );
    let (&statement, op) = checker
        .emissions
        .iter()
        .find(|(_, op)| {
            let span = checker.points[op.input].span;
            &source[span.start..span.end] == "7"
        })
        .unwrap();
    let block = op.targets[0].block;
    assert_eq!(index[&(block, None)].values[0].statement, statement);
    assert_eq!(
        reports.results[&block].1.slots,
        Some(vec![Sources::Candidates(
            index[&(block, None)].values.clone()
        )])
    );
}

#[test]
pub(crate) fn result_source_index_rejects_late_metadata_conflicts_without_spending_parts() {
    for fault in 0..8 {
        let (mut checker, mut reports) = checked("a:{->x:1};b:{->x:2}");
        let (&statement, op) = checker.emissions.last_key_value().unwrap();
        let emission = op.targets[0].id;
        let (owner, Effect::Emission(op)) = reports.effects.get_mut(&statement).unwrap() else {
            panic!()
        };
        match fault {
            0 => op.input = statement,
            1 => {
                op.composed =
                    Some(crate::check::dependencies::emissions::Composition { local: 0, count: 0 })
            }
            2 => op.targets[0].field = Some("other".into()),
            3 => op.control = !op.control,
            4 => op.initialized.clear(),
            5 => *owner += 1,
            6 => {
                checker.emission_sources.remove(&emission);
            }
            7 => {
                checker.emission_sources.insert(emission, (statement, 1));
            }
            _ => unreachable!(),
        }
        let effects = reports.effects.clone();
        let blocks = reports.blocks.clone();
        let mut parts = 4;
        assert!(
            checker
                .result_source_index(&reports, &mut parts, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(parts, 4);
        assert_eq!(reports.effects, effects);
        assert_eq!(reports.blocks, blocks);
    }
    for room in [3, 4] {
        let (mut checker, reports) = checked("a:{->x:1};b:{->x:2}");
        let mut parts = room;
        let result = checker.result_source_index(&reports, &mut parts, Span::default());
        assert_eq!(result.is_ok(), room == 4);
        assert_eq!(parts, if room == 4 { 0 } else { room });
    }
}
