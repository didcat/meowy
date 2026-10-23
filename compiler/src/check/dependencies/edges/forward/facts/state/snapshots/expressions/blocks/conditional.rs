use super::super::super::capture::tests::{capture, capture_with};
use super::boundaries::setup;
use super::*;

#[test]
pub(crate) fn scalar_conditional_blocks_publish_null_without_rewriting_slot_histories() {
    for source in [
        "f:(flag<boolean>){v:{|flag|->null};v}",
        "f:(flag<boolean>){v:'out{|flag|{'out->null;'out.leave()};1};v}",
        "f:(a<boolean>,b<boolean>){v:'out{|a|{'out->null;'out.leave()};|b|'out->null};v}",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let target = &checker.emissions.values().next().unwrap().targets[0];
        let parent = checker.bodies[&target.block].parent.unwrap();
        let value = snapshots.expressions[&parent].value.as_ref().unwrap();
        assert_eq!(value.domain, Domain::NULL);
        assert_eq!(
            (value.local, value.version, value.origin),
            (None, None, None)
        );
        assert!(!value.control);
        let Some(View::Known(state)) = snapshots.after(Port::BlockResult(target.block)) else {
            panic!()
        };
        let Data::Conditional { domain, origin } = &state.slot(target.block).unwrap().data else {
            panic!()
        };
        assert_eq!(*domain, Domain::NULL);
        assert_eq!(origin.is_some(), checker.emissions.len() == 1);
        for op in checker.emissions.values() {
            let target = &op.targets[0];
            let Some(View::Known(state)) = snapshots.after(Port::Emission(target.id)) else {
                panic!()
            };
            let emitted = snapshots.emissions[&target.id].value.as_ref().unwrap();
            assert!(emitted.origin.is_some());
            assert_eq!(
                state.slot(target.block).unwrap().data,
                Data::Initialized {
                    version: None,
                    domain: Some(Domain::NULL),
                    origin: emitted.origin,
                }
            );
        }
    }
}

#[test]
pub(crate) fn scalar_conditional_blocks_preserve_old_emissions_and_new_copy_identity() {
    let source = "f:(flag<boolean>){n:=null;v:{|flag|->n};copy:v;n=null;v;copy;n}";
    let (checker, _, _, snapshots) = capture(source);
    let target = &checker.emissions.values().next().unwrap().targets[0];
    let emitted = snapshots.emissions[&target.id].value.as_ref().unwrap();
    let parent = checker.bodies[&target.block].parent.unwrap();
    assert!(
        snapshots.expressions[&parent]
            .value
            .as_ref()
            .unwrap()
            .origin
            .is_none()
    );
    let Some(View::Known(state)) = snapshots.after(Port::BlockResult(target.block)) else {
        panic!()
    };
    assert_eq!(
        state.slot(target.block).unwrap().data,
        Data::Conditional {
            domain: Domain::NULL,
            origin: emitted.origin,
        }
    );
    let mut reads = BTreeMap::new();
    for &point in checker.local_reads.keys() {
        let span = checker.points[point].span;
        if span.start > source.rfind(";v;copy;n").unwrap() {
            let value = snapshots.expressions[&point].value.as_ref().unwrap();
            assert_eq!(value.domain, Domain::NULL);
            assert!(value.origin.is_some());
            assert_ne!(value.origin, emitted.origin);
            assert!(!value.control);
            reads.insert(&source[span.start..span.end], value.origin);
        }
    }
    assert_eq!(reads.len(), 3);
    assert_eq!(reads["v"], reads["copy"]);
    assert_ne!(reads["v"], reads["n"]);
}

#[test]
pub(crate) fn scalar_conditional_blocks_require_complete_inputs_and_keep_control_dependence() {
    for fault in 0..6 {
        let (checker, _, _, snapshots) =
            capture_with("f:(flag<boolean>){v:{|flag|->null};v}", |obs| match fault {
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
        let value = &snapshots.expressions[&point].value;
        assert_eq!(
            value.as_ref().map(|value| value.control),
            match fault {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            },
            "{fault}"
        );
        if let Some(value) = value {
            assert_eq!(value.domain, Domain::NULL);
            assert_eq!(value.origin, None);
        }
    }
}

#[test]
pub(crate) fn scalar_conditional_blocks_keep_unknown_tails_and_nullable_headers_unavailable() {
    for source in [
        "f:(flag<boolean>){v:{|flag|->null;1+2};v}",
        "g:(text<string>){};f:(flag<boolean>){v:{|flag|->null;g(\"cat\")};v}",
        "f:(flag<boolean>){v:'loop{|flag|->null;|false|'loop.restart()};v}",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let block = checker.emissions.values().next().unwrap().targets[0].block;
        let point = checker.bodies[&block].parent.unwrap();
        assert!(snapshots.expressions[&point].value.is_none(), "{source}");
    }
    let (checker, _, _, snapshots) = capture("f:(flag<boolean>){v:{|flag|->7};v}");
    let block = checker.emissions.values().next().unwrap().targets[0].block;
    let point = checker.bodies[&block].parent.unwrap();
    assert!(!snapshots.expressions.contains_key(&point));
}

#[test]
pub(crate) fn scalar_conditional_blocks_keep_origins_when_only_one_guard_arm_reaches_completion() {
    for flag in ["true", "false"] {
        let (checker, _, _, snapshots) = capture(&format!("v:{{|{flag}|->null}}"));
        let target = &checker.emissions.values().next().unwrap().targets[0];
        let body = &checker.bodies[&target.block];
        let point = body.parent.unwrap();
        let value = snapshots.expressions[&point].value.as_ref().unwrap();
        let origin = if flag == "true" {
            snapshots.emissions[&target.id]
                .value
                .as_ref()
                .unwrap()
                .origin
        } else {
            Some(Origin::Expression(body.owner, point))
        };
        assert_eq!(value.origin, origin);
        assert_eq!(value.domain, Domain::NULL);
        assert!(!value.control);
    }
}

#[test]
pub(crate) fn scalar_conditional_blocks_never_infer_null_from_opaque_or_incomplete_evidence() {
    for (data, missing) in [
        (Data::Partial, false),
        (Data::Unavailable, false),
        (
            Data::Initialized {
                version: None,
                domain: None,
                origin: None,
            },
            false,
        ),
        (
            Data::Conditional {
                domain: Domain::boolean(true),
                origin: None,
            },
            false,
        ),
        (
            Data::Conditional {
                domain: Domain::NULL,
                origin: None,
            },
            true,
        ),
    ] {
        setup(
            "f:(flag<boolean>){v:{|flag|->null};v}",
            |checker, snapshots, point, forward, obs| {
                let Port::BlockResult(block) = forward.input else {
                    panic!()
                };
                let Value::Known(id) = snapshots.rows[&forward.input].after else {
                    panic!()
                };
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
                if missing {
                    for port in [Port::BlockNormal(block), forward.input, Port::Normal(point)] {
                        snapshots.rows.get_mut(&port).unwrap().control = None;
                    }
                }
                snapshots
                    .block_expression(point, &forward, obs, &mut checker.flow, Span::default())
                    .unwrap();
                assert!(snapshots.expressions[&point].value.is_none());
            },
        );
    }
}
