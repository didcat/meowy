use super::super::capture::tests::capture;
use super::*;

#[test]
pub(crate) fn scalar_completion_storage_preserves_empty_scopes_tail_writes_and_direct_leaves() {
    for source in ["x:=1;{};{x=2};x", "x:=1;'out{x=2;'out.leave()};x"] {
        let (mut checker, _, _, snapshots) = capture(source);
        let point = *checker.local_reads.keys().next_back().unwrap();
        assert_eq!(
            snapshots.expressions[&point].value.as_ref().unwrap().domain,
            Domain::integer(2, &mut checker.flow, Span::default()).unwrap()
        );
        for body in checker.bodies.values() {
            if let Some(parent) = body.parent {
                assert_eq!(
                    snapshots.expressions[&parent]
                        .value
                        .as_ref()
                        .unwrap()
                        .domain,
                    Domain::NULL
                );
            }
        }
    }
}

#[test]
pub(crate) fn scalar_completion_storage_joins_guarded_leave_and_fallthrough_versions() {
    let (mut checker, _, _, snapshots) =
        capture("f:(flag<boolean>){x:=1;'out{|flag|'out.leave();x=2};x}");
    let point = *checker.local_reads.keys().next_back().unwrap();
    let expected = Domain::integer(1, &mut checker.flow, Span::default())
        .unwrap()
        .join(
            &Domain::integer(2, &mut checker.flow, Span::default()).unwrap(),
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    let value = snapshots.expressions[&point].value.as_ref().unwrap();
    assert_eq!(value.domain, expected);
    assert_eq!(value.version, None);
    assert_eq!(value.origin, None);
    assert!(!value.control);
}

#[test]
pub(crate) fn scalar_completion_storage_uses_qualified_boolean_paths_and_preserves_copies() {
    for (flag, expected) in [("true", 1), ("false", 2)] {
        let source = format!("x:=1;copy:x;'out{{|{flag}|'out.leave();x=2}};copy;x");
        let (mut checker, _, _, snapshots) = capture(&source);
        let point = *checker.local_reads.keys().next_back().unwrap();
        assert_eq!(
            snapshots.expressions[&point].value.as_ref().unwrap().domain,
            Domain::integer(expected, &mut checker.flow, Span::default()).unwrap()
        );
        for (&point, read) in &checker.local_reads {
            if read.local == 1 {
                assert_eq!(
                    snapshots.expressions[&point].value.as_ref().unwrap().domain,
                    Domain::integer(1, &mut checker.flow, Span::default()).unwrap()
                );
            }
        }
    }
}

#[test]
pub(crate) fn scalar_completion_storage_keeps_unknown_effects_and_restarts_unavailable() {
    for source in [
        "d:@\"debug\";x:=1;{d.print(\"tail\");x=2};x",
        "x:=1;'loop{|false|'loop.restart();x=2};x",
        "x:=1;{->field:2;x=2};x",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let point = *checker.local_reads.keys().next_back().unwrap();
        assert!(snapshots.expressions[&point].value.is_none(), "{source}");
    }
}

#[test]
pub(crate) fn scalar_stopped_paths_preserve_storage_across_nested_leaves_and_dead_tails() {
    for (source, expected) in [
        ("x:=1;'out{'out.leave();x=2};x", 1),
        ("x:=1;'out{|true|{'out.leave()};x=2};x", 1),
        ("x:=1;'out{{x=2;'out.leave();x=3};x=4};x", 2),
        ("x:=1;'out{'inner{'inner.leave();x=2};x=3};x", 3),
    ] {
        let (mut checker, _, _, snapshots) = capture(source);
        let point = checker.local_reads.keys().next_back().unwrap();
        assert_eq!(
            snapshots.expressions[point].value.as_ref().unwrap().domain,
            Domain::integer(expected, &mut checker.flow, Span::default()).unwrap(),
            "{source}"
        );
    }
}

#[test]
pub(crate) fn scalar_stopped_paths_keep_emitted_values_and_historical_copies_before_nested_exits() {
    let (mut checker, _, _, snapshots) =
        capture("x:=1;copy:x;v:'out{->x;{x=2;'out.leave();x=3};x=4};copy;x;v");
    let op = checker.emissions.values().next().unwrap();
    let parent = checker.bodies[&op.targets[0].block].parent.unwrap();
    let emitted = snapshots.emissions[&op.targets[0].id]
        .value
        .as_ref()
        .unwrap();
    let value = snapshots.expressions[&parent].value.as_ref().unwrap();
    assert_eq!(
        value.domain,
        Domain::integer(1, &mut checker.flow, Span::default()).unwrap()
    );
    assert_eq!(value.origin, emitted.origin);
    assert!(!value.control);
    for (&point, read) in &checker.local_reads {
        if read.local == 1 {
            let copy = snapshots.expressions[&point].value.as_ref().unwrap();
            assert_eq!(copy.domain, value.domain);
            assert_eq!(copy.origin, value.origin);
        }
    }
}

#[test]
pub(crate) fn scalar_stopped_paths_keep_derived_and_missing_guard_history() {
    for control in [Some(false), Some(true), None] {
        let (checker, _, _, snapshots) = super::super::capture::tests::capture_with(
            "v:'out{->7;|true|{'out.leave()};1}",
            |obs| {
                for input in obs.guards.values_mut() {
                    input.control = control;
                }
            },
        );
        let op = checker.emissions.values().next().unwrap();
        let parent = checker.bodies[&op.targets[0].block].parent.unwrap();
        assert_eq!(
            snapshots.expressions[&parent]
                .value
                .as_ref()
                .map(|value| value.control),
            control
        );
        assert!(
            !snapshots.emissions[&op.targets[0].id]
                .value
                .as_ref()
                .unwrap()
                .control
        );
    }
}

#[test]
pub(crate) fn scalar_stopped_paths_join_distinct_live_exit_versions_across_function_owners() {
    let source = "f:(flag<boolean>){x:=1;v:'out{->7;|flag|{x=2;'out.leave()};x=3};x;v}";
    let (mut checker, _, _, snapshots) = capture(source);
    let at = source.rfind("x;v").unwrap();
    let (&point, _) = checker
        .local_reads
        .iter()
        .find(|(point, _)| checker.points[**point].span == Span::new(at, at + 1))
        .unwrap();
    let expected = Domain::integer(2, &mut checker.flow, Span::default())
        .unwrap()
        .join(
            &Domain::integer(3, &mut checker.flow, Span::default()).unwrap(),
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    let value = snapshots.expressions[&point].value.as_ref().unwrap();
    assert_eq!(value.domain, expected);
    assert_eq!(value.version, None);
    assert_eq!(value.origin, None);
    let op = checker.emissions.values().next().unwrap();
    let parent = checker.bodies[&op.targets[0].block].parent.unwrap();
    let completed = snapshots.expressions[&parent].value.as_ref().unwrap();
    assert_eq!(
        completed.domain,
        Domain::integer(7, &mut checker.flow, Span::default()).unwrap()
    );
    assert_eq!(
        completed.origin,
        snapshots.emissions[&op.targets[0].id]
            .value
            .as_ref()
            .unwrap()
            .origin
    );
    assert_eq!(snapshots.expressions[&parent].owner, 1);
}

#[test]
pub(crate) fn scalar_completion_storage_requires_known_control_and_original_owner_observations() {
    let (mut checker, program, reports, snapshots) = capture("x:=1;'out{x=2;'out.leave()};x");
    let (_, _, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, parts) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    let (graph, parts) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let (observations, parts) = checker
        .scalar_scope_observations(
            &reports,
            &graph,
            observations::Observations::new(&definitions),
            Span::default(),
            parts,
        )
        .unwrap();
    let (mut observations, _) = checker
        .scalar_completion_observations(&reports, &graph, observations, Span::default(), parts)
        .unwrap();
    let Port::Leave(block) = checker.scope_exits.values().next().unwrap().edge.to else {
        panic!()
    };
    let before = snapshots.rows[&Port::BlockNormal(block)].before;
    let saved = format!("{snapshots:?}");
    for control in [Some(false), Some(true), None] {
        observations.scopes.get_mut(&block).unwrap().control = control;
        observations.completion.leaves.insert(block, control);
        for port in [
            Port::Leave(block),
            Port::BlockNormal(block),
            Port::BlockResult(block),
        ] {
            assert_eq!(
                snapshots
                    .complete(
                        port,
                        before,
                        &observations,
                        &mut checker.flow,
                        Span::default()
                    )
                    .unwrap(),
                control.map_or(Value::Missing, |_| before)
            );
            assert_eq!(
                snapshots
                    .complete(
                        port,
                        Value::Missing,
                        &observations,
                        &mut checker.flow,
                        Span::default()
                    )
                    .unwrap(),
                Value::Missing
            );
        }
    }
    observations.scopes.get_mut(&block).unwrap().owner += 1;
    assert!(
        snapshots
            .complete(
                Port::BlockNormal(block),
                before,
                &observations,
                &mut checker.flow,
                Span::default()
            )
            .unwrap_err()
            .message
            .contains("identity")
    );
    observations.scopes.get_mut(&block).unwrap().owner -= 1;
    observations.scopes.get_mut(&block).unwrap().result = false;
    assert!(
        snapshots
            .complete(
                Port::BlockResult(block),
                before,
                &observations,
                &mut checker.flow,
                Span::default()
            )
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{snapshots:?}"), saved);
}
