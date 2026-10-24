use super::super::super::capture::tests::{capture, capture_with};
use super::boundaries::setup;
use super::*;

#[test]
pub(crate) fn nullable_block_values_preserve_control_and_require_complete_inputs() {
    for literal in ["true", "7"] {
        for fault in 0..6 {
            let source = format!("f:(flag<boolean>){{v:{{|flag|->{literal}}};v}}");
            let (checker, _, _, snapshots) = capture_with(&source, |obs| match fault {
                0 => {}
                1 | 2 => {
                    for input in obs.guards.values_mut() {
                        input.control = if fault == 1 { Some(true) } else { None };
                    }
                }
                3 => obs.emissions.clear(),
                4 => obs.slots.clear(),
                5 => obs.completion.absent.clear(),
                _ => unreachable!(),
            });
            let block = checker.emissions.values().next().unwrap().targets[0].block;
            let point = checker.bodies[&block].parent.unwrap();
            let value = snapshots.expressions[&point].value.as_ref();
            assert_eq!(
                value.map(|value| value.control),
                match fault {
                    0 => Some(false),
                    1 => Some(true),
                    _ => None,
                },
                "{source} {fault}"
            );
            if let Some(value) = value {
                assert!(value.domain.null && value.domain.values.is_some());
                assert_eq!(value.origin, None);
            }
        }
    }
}

#[test]
pub(crate) fn nullable_block_values_do_not_materialize_null_from_opaque_or_incompatible_slots() {
    for data in [
        None,
        Some(Data::Partial),
        Some(Data::Unavailable),
        Some(Data::Initialized {
            version: None,
            domain: None,
            origin: None,
        }),
        Some(Data::Conditional {
            domain: Domain::boolean(true),
            origin: None,
        }),
        Some(Data::Conditional {
            domain: Domain::integer(300, &mut Flow::new(), Span::default()).unwrap(),
            origin: None,
        }),
    ] {
        setup(
            "f:(flag<boolean>,n<int8>){v:{|flag|->n}}",
            |checker, snapshots, point, forward, obs| {
                let Port::BlockResult(block) = forward.input else {
                    panic!()
                };
                let Value::Known(id) = snapshots.rows[&forward.input].after else {
                    panic!()
                };
                if let Some(data) = data {
                    snapshots.states[id]
                        .put_slot(
                            block,
                            &Cell {
                                data,
                                control: true,
                            },
                            &mut checker.flow,
                            Span::default(),
                            MAX_EDGES,
                        )
                        .unwrap();
                } else {
                    snapshots.states[id] = State::new(
                        forward.owner,
                        crate::flow::TRUE,
                        &mut checker.flow,
                        Span::default(),
                    )
                    .unwrap();
                }
                snapshots
                    .block_expression(point, &forward, obs, &mut checker.flow, Span::default())
                    .unwrap();
                assert!(snapshots.expressions[&point].value.is_none());
            },
        );
    }
}

#[test]
pub(crate) fn nullable_block_values_keep_conversion_unknown_tail_and_restart_boundaries() {
    for source in [
        "v<boolean><null>:{|true|->true};v",
        "v<int8><null>:{|true|->7};v",
        "f:(flag<boolean>){v:{|flag|->7;1+2};v}",
        "g:(text<string>){};f:(flag<boolean>){v:{|flag|->true;g(\"cat\")};v}",
        "f:(flag<boolean>){v:'loop{|flag|->7;|false|'loop.restart()};v}",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let body = checker
            .bodies
            .values()
            .find(|body| body.parent.is_some() && body.scalar.is_some_and(|scalar| scalar.nullable))
            .unwrap();
        assert!(
            snapshots.expressions[&body.parent.unwrap()].value.is_none(),
            "{source}"
        );
    }
}

#[test]
pub(crate) fn nullable_block_publication_rejects_header_kind_width_and_nullability_drift() {
    use crate::check::dependencies::ScalarKind;

    for fault in 0..4 {
        setup(
            "f:(flag<boolean>,n<int8>){v:{|flag|->n}}",
            |checker, snapshots, point, forward, obs| {
                let Port::BlockResult(block) = forward.input else {
                    panic!()
                };
                let scope = obs.scopes.get_mut(&block).unwrap();
                match fault {
                    0 => scope.scalar.nullable = false,
                    1 => scope.scalar.kind = ScalarKind::Bool,
                    2 => {
                        scope.scalar.kind = ScalarKind::Int {
                            bits: 8,
                            signed: false,
                        }
                    }
                    3 => obs.slots.get_mut(&block).unwrap().declared.null = false,
                    _ => unreachable!(),
                }
                let saved = format!("{snapshots:?}{obs:?}");
                assert!(
                    snapshots
                        .block_expression(point, &forward, obs, &mut checker.flow, Span::default())
                        .unwrap_err()
                        .message
                        .contains("identity")
                );
                assert_eq!(format!("{snapshots:?}{obs:?}"), saved);
            },
        );
    }
}
