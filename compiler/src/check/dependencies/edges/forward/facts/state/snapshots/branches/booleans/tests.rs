use super::super::super::capture::tests::capture;
use super::*;

pub(super) fn setup() -> (Checker, Snapshots, Edge, Value) {
    let (checker, _, _, snapshots) = capture("f:(b<boolean>){b&&b}");
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
pub(crate) fn scalar_boolean_edges_refine_current_reads_preserve_versions_and_rejoin() {
    for input in ["b", "(b)", "(b)~<boolean>"] {
        for op in ["&&", "||"] {
            let (checker, program, _, snapshots) =
                capture(&format!("f:(b<boolean>){{{input}{op}b}}"));
            let local = program.functions[0].params[0];
            let read = *checker
                .local_reads
                .iter()
                .rfind(|(_, read)| read.local == local)
                .unwrap()
                .0;
            let value = snapshots.expressions[&read].value.as_ref().unwrap();
            let (&point, fact) = snapshots.guards.iter().next().unwrap();
            let original = snapshots.expressions[&fact.input.source.input]
                .value
                .as_ref()
                .unwrap();
            assert_eq!(original.domain.values, Some(Values::Bool(3)));
            assert_eq!(value.domain, Domain::boolean(op == "&&"));
            assert_eq!(value.version, original.version);
            assert_eq!(value.origin, original.origin);
            let Some(View::Known(after)) = snapshots.after(Port::Normal(fact.input.point)) else {
                panic!()
            };
            let Data::Initialized {
                domain,
                version,
                origin,
            } = &after.get(local).unwrap().data
            else {
                panic!()
            };
            assert_eq!(domain.as_ref(), Some(&original.domain));
            assert_eq!(*version, original.version);
            assert_eq!(*origin, original.origin);
            let Some(View::Known(before)) = snapshots.after(Port::Normal(point)) else {
                panic!()
            };
            assert_eq!(before.get(local), after.get(local));
        }
    }
}

#[test]
pub(crate) fn scalar_boolean_edges_do_not_reapply_old_versions_after_writes() {
    let (checker, _, _, snapshots) = capture("f:(b<boolean>){x:=b;old:x;|x|x=false;after:x;old}");
    let x = checker
        .operations
        .values()
        .find(|op| op.kind == crate::check::dependencies::OperationKind::Write)
        .unwrap()
        .local;
    let reads = checker
        .local_reads
        .iter()
        .filter(|(_, read)| read.local == x)
        .map(|(point, _)| snapshots.expressions[point].value.as_ref().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(reads.len(), 3);
    assert_eq!(reads[0].domain.values, Some(Values::Bool(3)));
    assert_eq!(reads[2].domain, Domain::boolean(false));
    assert_eq!(reads[2].version, None);
    let copy = *checker.local_reads.keys().next_back().unwrap();
    assert_eq!(
        snapshots.expressions[&copy]
            .value
            .as_ref()
            .unwrap()
            .domain
            .values,
        Some(Values::Bool(3))
    );
}

#[test]
pub(crate) fn scalar_boolean_edges_keep_merged_versions_conservative() {
    let (checker, _, _, snapshots) = capture("f:(b<boolean>,c<boolean>){x:=b;|c|x=true;|x|x}");
    let x = checker
        .operations
        .values()
        .find(|op| op.kind == crate::check::dependencies::OperationKind::Bind)
        .unwrap()
        .local;
    let fact = snapshots
        .guards
        .values()
        .max_by_key(|fact| fact.input.point)
        .unwrap();
    assert_eq!(
        snapshots.expressions[&fact.input.source.input]
            .value
            .as_ref()
            .unwrap()
            .version,
        None
    );
    for arm in fact.input.arms {
        let Some(View::Known(state)) = snapshots.before(Port::Entry(arm)) else {
            panic!()
        };
        assert!(
            matches!(&state.get(x).unwrap().data, Data::Initialized { version: None, domain: Some(domain), .. } if domain.values == Some(Values::Bool(3)))
        );
    }
}

#[test]
pub(crate) fn scalar_boolean_edges_reject_stale_source_values_and_versions_atomically() {
    for fault in 0..5 {
        let (mut checker, mut snapshots, edge, before) = setup();
        let Port::Normal(point) = edge.from else {
            panic!()
        };
        let input = snapshots.guards[&point].input.source.input;
        let scalar = snapshots
            .expressions
            .get_mut(&input)
            .unwrap()
            .value
            .as_mut()
            .unwrap();
        match fault {
            0 => scalar.control = true,
            1 => scalar.domain = Domain::boolean(true),
            2 => scalar.version.as_mut().unwrap().local = usize::MAX,
            3 => scalar.origin = None,
            4 => snapshots.rows.get_mut(&Port::Normal(input)).unwrap().after = Value::Missing,
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
pub(crate) fn scalar_boolean_edges_bound_new_cell_paths_and_preserve_source_snapshots() {
    let (mut checker, mut snapshots, edge, before) = setup();
    let start = checker.flow.work;
    let room = snapshots.room();
    let original = format!("{:?}{:?}", snapshots.rows, snapshots.expressions);
    let next = snapshots
        .branch(edge, before, &mut checker.flow, Span::default())
        .unwrap();
    assert_ne!(next, before);
    let cost = checker.flow.work - start;
    let used = room - snapshots.room();
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        snapshots.parts = room;
        checker.flow.full = false;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        let len = snapshots.states.len();
        assert_eq!(
            snapshots
                .branch(edge, before, &mut checker.flow, Span::default())
                .is_ok(),
            pass
        );
        assert_eq!(snapshots.states.len(), len + usize::from(pass));
        assert_eq!(snapshots.room(), if pass { 0 } else { room });
        assert_eq!(
            format!("{:?}{:?}", snapshots.rows, snapshots.expressions),
            original
        );
    }
}
