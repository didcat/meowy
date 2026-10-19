use super::super::versions::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_ports_retain_owner_scope_source_and_independent_entries() {
    let (mut checker, program, reports) = context(
        "flag:false;'out{'inner{|flag|'out->n:1;'inner.leave()}};unused:(n<int32>){copy:n}",
    );
    let (_, _, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, _) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    assert_eq!(
        ports.len(),
        reports
            .entries
            .values()
            .map(|(_, walk)| walk.ports.len())
            .sum()
    );
    assert_eq!(entries.len(), 2);
    for (&owner, (entry, state)) in &entries {
        assert_eq!(ports[entry].owner, owner);
        assert_eq!(state.owner(), owner);
        assert_eq!(*entry, Port::BlockEntry(ports[entry].block));
    }
    for (port, site) in &ports {
        assert_eq!(checker.bodies[&site.block].owner, site.owner);
        if let Port::Operation(point) = port {
            assert_eq!(site.span, checker.points[*point].span);
            assert_eq!(Some(site.block), checker.points[*point].block);
        }
        if let Port::Emission(id) = port {
            let (point, index) = checker.emission_sources[id];
            assert_eq!(site.block, checker.emissions[&point].targets[index].block);
        }
    }
}

#[test]
pub(crate) fn scalar_ports_do_not_create_normal_stages_after_stopped_operations() {
    let (mut checker, program, reports) =
        context("d:@\"debug\";stop<never>:(){d.panic(\"stop\")};stop();late:9");
    let (_, _, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, _) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    let call = checker
        .invocations
        .values()
        .find(|call| call.owner == 0)
        .unwrap();
    assert!(ports.contains_key(&Port::Operation(call.point)));
    assert!(!ports.contains_key(&Port::Normal(call.point)));
    let late = checker.operations.last_key_value().unwrap().0;
    assert!(!ports.contains_key(&Port::Operation(*late)));
}

#[test]
pub(crate) fn scalar_ports_reject_missing_foreign_duplicate_and_unscoped_sites() {
    for fault in 0..5 {
        let (mut checker, program, mut reports) = context("x:1;f:(n<int32>){n}");
        let (_, _, mut entries, parts) = checker
            .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
            .unwrap();
        match fault {
            0 => {
                entries.remove(&1);
            }
            1 => {
                entries.get_mut(&1).unwrap().0 = entries[&0].0;
            }
            2 => {
                entries.get_mut(&1).unwrap().1 =
                    state::State::new(0, crate::flow::TRUE, &mut checker.flow, Span::default())
                        .unwrap();
            }
            3 => {
                let walk = &mut reports.entries.get_mut(&0).unwrap().1;
                walk.ports.push(walk.ports[0]);
            }
            4 => {
                let point = *checker.operations.first_key_value().unwrap().0;
                checker.points[point].block = None;
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{entries:?}");
        assert_eq!(
            checker
                .scalar_ports(&reports, &entries, Span::default(), parts)
                .unwrap_err()
                .code,
            "B001"
        );
        assert_eq!(format!("{reports:?}{entries:?}"), before);
    }
}

#[test]
pub(crate) fn scalar_ports_bound_exact_work_and_payload_before_publication() {
    let (mut checker, program, reports) = context("flag:false;|flag|x:1;unused:(n<int32>){}");
    let (_, _, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let start = checker.flow.work;
    let expected = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    let cost = checker.flow.work - start;
    let used = parts - expected.1;
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = checker.scalar_ports(&reports, &entries, Span::default(), room);
        assert_eq!(result.is_ok(), pass);
        if let Ok((ports, left)) = result {
            assert_eq!(ports, expected.0);
            assert_eq!(left, 0);
        }
    }
}
