use super::super::capture::tests::{capture, capture_with};
use super::*;
use crate::check::dependencies::bodies::completion::Shape;

#[test]
pub(crate) fn primary_field_emissions_preserve_saved_slots_and_tail_writes() {
    for source in [
        "x:=7;-{->x;->tag:false;x=9}",
        "x:=7;-{->tag:false;->x;x=9}",
        "x:=7;-'out{->x;{ 'out->tag:false };x=9}",
    ] {
        let (mut checker, _, _, snapshots) = capture(source);
        let (&block, body) = checker
            .bodies
            .iter()
            .find(|(_, body)| matches!(body.completion.result, Shape::Record { .. }))
            .unwrap();
        let Some(View::Known(state)) = snapshots.after(Port::BlockResult(block)) else {
            panic!("{source}")
        };
        let Data::Initialized {
            domain: Some(domain),
            origin,
            version,
        } = &state.slot(block).unwrap().data
        else {
            panic!()
        };
        assert_eq!(
            *domain,
            Domain::integer(7, &mut checker.flow, Span::default()).unwrap()
        );
        assert!(origin.is_some());
        assert_eq!(*version, None);
        assert!(!snapshots.expressions.contains_key(&body.parent.unwrap()));
        let Data::Initialized {
            domain: Some(domain),
            ..
        } = &state.get(0).unwrap().data
        else {
            panic!()
        };
        assert_eq!(
            *domain,
            Domain::integer(9, &mut checker.flow, Span::default()).unwrap()
        );
        for op in checker
            .emissions
            .values()
            .filter(|op| op.targets[0].field.is_some())
        {
            let port = Port::Emission(op.targets[0].id);
            assert_eq!(snapshots.before(port), snapshots.after(port));
            assert!(!snapshots.emissions.contains_key(&op.targets[0].id));
        }
    }
}

#[test]
pub(crate) fn primary_field_emissions_preserve_unknown_inputs_and_unsupported_layouts() {
    for source in [
        "x:=7;-{->x;->tag:1+2}",
        "x:=7;-{->x;->tag:=false}",
        "x:=7;-{->x;->tag:&x}",
        "x:=7;-{->x;->tag:\"cat\"}",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let (&block, _) = checker
            .bodies
            .iter()
            .find(|(_, body)| matches!(body.completion.result, Shape::Record { .. }))
            .unwrap();
        assert_eq!(
            snapshots.after(Port::BlockResult(block)),
            Some(View::Missing),
            "{source}"
        );
    }
}

#[test]
pub(crate) fn primary_field_emissions_carry_field_control_without_rewriting_primary_control() {
    let (checker, _, _, snapshots) = capture_with("-{->7;->tag:false}", |obs| {
        for (port, forward) in &mut obs.forwarded {
            if matches!(port, Port::Emission(_)) {
                forward.control = true;
            }
        }
    });
    let primary = checker
        .emissions
        .values()
        .find(|op| op.targets[0].field.is_none())
        .unwrap();
    let field = checker
        .emissions
        .values()
        .find(|op| op.targets[0].field.is_some())
        .unwrap();
    assert!(
        !snapshots.emissions[&primary.targets[0].id]
            .value
            .as_ref()
            .unwrap()
            .control
    );
    assert_eq!(
        snapshots.rows[&Port::Emission(field.targets[0].id)].control,
        Some(true)
    );
}
