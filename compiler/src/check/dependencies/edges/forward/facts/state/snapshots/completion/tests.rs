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
                assert!(!snapshots.expressions.contains_key(&parent));
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
pub(crate) fn scalar_completion_storage_keeps_unknown_effects_dead_tails_and_restarts_unavailable()
{
    for source in [
        "d:@\"debug\";x:=1;{d.print(\"tail\");x=2};x",
        "x:=1;'out{'out.leave();1};x",
        "x:=1;'loop{|false|'loop.restart();x=2};x",
        "x:=1;{->field:2;x=2};x",
        "x:=1;'out{|true|{'out.leave()};x=2};x",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let point = *checker.local_reads.keys().next_back().unwrap();
        assert!(snapshots.expressions[&point].value.is_none(), "{source}");
    }
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
