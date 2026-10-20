use super::super::capture::tests::capture;
use super::*;
use crate::check::dependencies::ScalarLiteral;

#[test]
pub(crate) fn scalar_expression_literals_keep_values_owners_and_distinct_origins() {
    let (mut checker, _, _, snapshots) = capture("7;7;false;null;1.5;\"cat\";f:(){42}");
    let mut equal = Vec::new();
    for (&point, leaf) in &checker.scalar_leaves {
        let fact = &snapshots.expressions[&point];
        assert_eq!(fact.owner, leaf.owner);
        assert_eq!(fact.value.is_some(), leaf.value.is_some());
        if let Some(value) = &fact.value {
            assert_eq!(
                value.domain,
                Domain::literal(leaf.value.unwrap(), &mut checker.flow, Span::default()).unwrap()
            );
            assert_eq!(value.origin, Some(Origin::Expression(leaf.owner, point)));
            assert_eq!(value.version, None);
            assert_eq!(value.local, None);
            assert_eq!(value.control, leaf.control);
            if leaf.value == Some(ScalarLiteral::Int(7)) {
                equal.push(value);
            }
        }
    }
    assert_eq!(equal.len(), 2);
    assert_eq!(equal[0].domain, equal[1].domain);
    assert_ne!(equal[0].origin, equal[1].origin);
}

#[test]
pub(crate) fn scalar_expression_literals_keep_unavailable_and_bottom_separate() {
    let (checker, _, _, snapshots) =
        capture("x:1;5+6;2;d:@\"debug\";d.panic(\"stop\");late:9;f:(n<never>){7}");
    for (&point, leaf) in &checker.scalar_leaves {
        match leaf.value {
            Some(ScalarLiteral::Int(1)) => assert!(snapshots.expressions[&point].value.is_some()),
            Some(ScalarLiteral::Int(2)) => assert!(snapshots.expressions[&point].value.is_none()),
            Some(ScalarLiteral::Int(9)) => assert!(!snapshots.expressions.contains_key(&point)),
            Some(ScalarLiteral::Int(7)) => {
                let value = snapshots.expressions[&point].value.as_ref().unwrap();
                assert_eq!(value.domain, Domain::BOTTOM);
                assert_eq!(value.origin, None);
                assert_eq!(value.local, None);
            }
            _ => {}
        }
    }
}

#[test]
pub(crate) fn scalar_expression_records_require_result_stages_and_preserve_inputs_on_failure() {
    let (mut checker, _, mut reports, mut snapshots) = capture("7");
    let point = *checker.scalar_leaves.first_key_value().unwrap().0;
    let expected = snapshots.expressions.remove(&point).unwrap();
    let definitions = definitions::Definitions::default();
    let (owner, Effect::Scalar(observed)) = reports.effects.get_mut(&point).unwrap() else {
        panic!()
    };
    observed.control = true;
    let owner = *owner;
    let observations = observations::Observations {
        sources: [
            (Port::Operation(point), &reports.effects[&point]),
            (Port::Normal(point), &reports.effects[&point]),
        ]
        .into_iter()
        .collect(),
        ..observations::Observations::new(&definitions)
    };
    snapshots
        .expression(
            Port::Operation(point),
            &observations,
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    assert!(snapshots.expressions.is_empty());
    snapshots.parts = MAX_EDGES;
    let start = checker.flow.work;
    snapshots
        .expression(
            Port::Normal(point),
            &observations,
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    let cost = checker.flow.work - start;
    assert_eq!(MAX_EDGES - snapshots.room(), expected.weight());
    let current = snapshots.expressions.remove(&point).unwrap();
    assert!(current.value.as_ref().unwrap().control);
    assert_eq!(current.owner, owner);
    let before = format!("{:?}{:?}", snapshots.rows, snapshots.states);
    for (room, extra, pass) in [
        (expected.weight() - 1, 0, false),
        (expected.weight(), 1, false),
        (expected.weight(), 0, true),
    ] {
        snapshots.parts = room;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = snapshots.expression(
            Port::Normal(point),
            &observations,
            &mut checker.flow,
            Span::default(),
        );
        assert_eq!(result.is_ok(), pass);
        assert_eq!(snapshots.room(), if pass { 0 } else { room });
        assert_eq!(
            snapshots.expressions.get(&point),
            if pass { Some(&current) } else { None }
        );
        assert_eq!(
            format!("{:?}{:?}", snapshots.rows, snapshots.states),
            before
        );
    }
}
