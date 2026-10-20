use super::super::capture::tests::capture;
use super::*;

pub(super) fn setup() -> (Checker, Snapshots, Edge, Value) {
    let (checker, _, _, snapshots) = capture("flag:false;|flag|1");
    let (&point, fact) = snapshots.guards.iter().next().unwrap();
    let edge = Edge::new(
        Port::Normal(point),
        Port::Entry(fact.input.arms[0]),
        Route::True,
    );
    let before = snapshots.rows[&edge.from].after;
    (checker, snapshots, edge, before)
}

#[test]
pub(crate) fn scalar_branches_select_constant_edges_and_join_independent_matcher_writes() {
    for (condition, expected) in [("true", 1), ("false", 0)] {
        let source = format!("x:=0;|{condition}|x=1;after:x");
        let (mut checker, _, _, snapshots) = capture(&source);
        let read = *checker.local_reads.keys().next().unwrap();
        assert_eq!(
            snapshots.expressions[&read].value.as_ref().unwrap().domain,
            Domain::integer(expected, &mut checker.flow, Span::default()).unwrap()
        );
        let (&point, fact) = snapshots.guards.iter().next().unwrap();
        let source = snapshots.rows[&Port::Normal(point)].after;
        assert_ne!(snapshots.view(source).unwrap(), View::Missing);
        for (arm, allowed) in [(0, condition == "true"), (1, condition == "false")] {
            let Some(View::Known(state)) = snapshots.before(Port::Entry(fact.input.arms[arm]))
            else {
                panic!()
            };
            assert_eq!(state.reach != FALSE, allowed);
        }
    }
    let (mut checker, _, _, snapshots) = capture("x:=0;|true|x=1;|true|x=2;after:x");
    let read = *checker.local_reads.keys().next().unwrap();
    assert_eq!(
        snapshots.expressions[&read].value.as_ref().unwrap().domain,
        Domain::integer(2, &mut checker.flow, Span::default()).unwrap()
    );
}

#[test]
pub(crate) fn scalar_branches_route_short_circuits_without_mutating_condition_history() {
    for (op, left, rhs_live) in [
        ("&&", "false", false),
        ("&&", "true", true),
        ("||", "false", true),
        ("||", "true", false),
    ] {
        let (_, _, _, snapshots) = capture(&format!("{left}{op}false"));
        let (&point, fact) = snapshots.guards.iter().next().unwrap();
        let rhs = fact.input.arms[usize::from(op == "||")];
        let Some(View::Known(state)) = snapshots.before(Port::Entry(rhs)) else {
            panic!()
        };
        assert_eq!(state.reach != FALSE, rhs_live);
        let Some(View::Known(state)) = snapshots.after(Port::Normal(point)) else {
            panic!()
        };
        assert_ne!(state.reach, FALSE);
        assert_eq!(
            snapshots.expressions[&fact.input.source.input]
                .value
                .as_ref()
                .unwrap()
                .domain,
            Domain::boolean(left == "true")
        );
    }
}

#[test]
pub(crate) fn scalar_branches_keep_unknown_dependencies_and_unreached_predecessors_conservative() {
    for authority in [Authority::Unknown, Authority::Dependent] {
        let (mut checker, mut snapshots, edge, before) = setup();
        let Port::Normal(point) = edge.from else {
            panic!()
        };
        snapshots.guards.get_mut(&point).unwrap().authority = authority;
        if authority == Authority::Dependent {
            snapshots.guards.get_mut(&point).unwrap().input.control = Some(true);
        }
        let room = snapshots.room();
        assert_eq!(
            snapshots
                .branch(edge, before, &mut checker.flow, Span::default())
                .unwrap(),
            if authority == Authority::Dependent {
                before
            } else {
                Value::Missing
            }
        );
        assert_eq!(snapshots.room(), room);
        snapshots.guards.clear();
        assert_eq!(
            snapshots
                .branch(edge, before, &mut checker.flow, Span::default())
                .unwrap(),
            Value::Missing
        );
    }
    let (checker, _, _, snapshots) =
        capture("d:@\"debug\";stop<never>:(){d.panic(\"stop\")};x:=0;|false|stop();after:x");
    let read = *checker.local_reads.keys().next().unwrap();
    assert!(snapshots.expressions[&read].value.is_none());
}

#[test]
pub(crate) fn scalar_branches_do_not_restore_facts_under_incomplete_guard_evidence() {
    for source in [
        "x:=0;flag:=false;|(!flag)|x=1;after:x",
        "x:=0;flag:1==1;|flag|x=1;after:x",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let read = *checker.local_reads.keys().next_back().unwrap();
        assert!(snapshots.expressions[&read].value.is_none());
    }
    let (mut checker, mut snapshots, edge, before) = setup();
    let Port::Normal(point) = edge.from else {
        panic!()
    };
    snapshots.guards.get_mut(&point).unwrap().authority = Authority::Dependent;
    assert_eq!(
        snapshots
            .branch(edge, before, &mut checker.flow, Span::default())
            .unwrap(),
        Value::Missing
    );
}

#[test]
pub(crate) fn scalar_branches_reject_changed_arms_owners_histories_and_authority() {
    for fault in 0..5 {
        let (mut checker, mut snapshots, mut edge, before) = setup();
        let Port::Normal(point) = edge.from else {
            panic!()
        };
        let fact = snapshots.guards.get_mut(&point).unwrap();
        match fault {
            0 => edge.to = Port::Entry(fact.input.arms[1]),
            1 => fact.input.source.owner += 1,
            2 => snapshots.rows.get_mut(&edge.from).unwrap().after = Value::Missing,
            3 => fact.input.control = Some(true),
            4 => fact.authority = Authority::Boolean(4),
            _ => unreachable!(),
        }
        let len = snapshots.states.len();
        let room = snapshots.room();
        assert!(
            snapshots
                .branch(edge, before, &mut checker.flow, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(snapshots.states.len(), len);
        assert_eq!(snapshots.room(), room);
    }
}

#[test]
pub(crate) fn scalar_branches_reserve_exact_bottom_state_payload_and_work() {
    let (mut checker, mut snapshots, edge, before) = setup();
    let start = checker.flow.work;
    let room = snapshots.room();
    let result = snapshots
        .branch(edge, before, &mut checker.flow, Span::default())
        .unwrap();
    assert!(matches!(snapshots.view(result), Some(View::Known(state)) if state.reach == FALSE));
    let work = checker.flow.work - start;
    let used = room - snapshots.room();
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        snapshots.parts = room;
        checker.flow.full = false;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + extra;
        let len = snapshots.states.len();
        assert_eq!(
            snapshots
                .branch(edge, before, &mut checker.flow, Span::default())
                .is_ok(),
            pass
        );
        assert_eq!(snapshots.states.len(), len + usize::from(pass));
        assert_eq!(snapshots.room(), if pass { 0 } else { room });
    }
}
