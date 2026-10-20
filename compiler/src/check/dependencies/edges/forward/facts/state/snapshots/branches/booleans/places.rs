use super::super::super::capture::tests::capture;
use super::*;

pub(super) fn setup() -> (Checker, Snapshots, Edge, Value, PointId, hir::LocalId) {
    let (checker, _, _, snapshots) = capture("f:(a<boolean>,b<boolean>){x:=a;|b|x=true;|x|x}");
    let (&point, fact) = snapshots
        .guards
        .iter()
        .max_by_key(|(_, fact)| fact.input.point)
        .unwrap();
    let input = fact.input.source.input;
    let scalar = snapshots.expressions[&input].value.as_ref().unwrap();
    let local = scalar.local.unwrap();
    assert_eq!(scalar.version, None);
    let edge = Edge::new(
        Port::Normal(point),
        Port::Entry(fact.input.arms[0]),
        Route::True,
    );
    let before = snapshots.rows[&edge.from].after;
    (checker, snapshots, edge, before, input, local)
}

#[test]
pub(crate) fn scalar_boolean_places_refine_joined_reads_through_transparent_wrappers() {
    for input in ["x", "(x)", "((x)~<boolean>)"] {
        for op in ["&&", "||"] {
            let (checker, _, _, snapshots) = capture(&format!(
                "f:(a<boolean>,b<boolean>){{x:=a;|b|x=true;{input}{op}x}}"
            ));
            let read = *checker.local_reads.keys().next_back().unwrap();
            let value = snapshots.expressions[&read].value.as_ref().unwrap();
            assert_eq!(value.domain, Domain::boolean(op == "&&"));
            assert_eq!(value.local, Some(checker.local_reads[&read].local));
            assert_eq!(value.version, None);
            assert_eq!(value.origin, None);
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
                    .domain
                    .values,
                Some(Values::Bool(3))
            );
        }
    }
}

#[test]
pub(crate) fn scalar_boolean_places_keep_equal_merged_cells_and_old_copies_distinct() {
    for other in ["y:=b;|c|y=true", "y:x"] {
        let source = format!("f:(a<boolean>,b<boolean>,c<boolean>){{x:=a;|c|x=true;{other};|x|y}}");
        let (checker, _, _, snapshots) = capture(&source);
        let read = *checker.local_reads.keys().next_back().unwrap();
        let copy = snapshots.expressions[&read].value.as_ref().unwrap();
        let fact = snapshots
            .guards
            .values()
            .max_by_key(|fact| fact.input.point)
            .unwrap();
        let original = snapshots.expressions[&fact.input.source.input]
            .value
            .as_ref()
            .unwrap();
        assert_ne!(copy.local, original.local);
        assert_eq!(copy.domain.values, Some(Values::Bool(3)));
        let Some(View::Known(state)) = snapshots.before(Port::Entry(fact.input.arms[0])) else {
            panic!()
        };
        assert!(
            matches!(&state.get(original.local.unwrap()).unwrap().data, Data::Initialized { version: None, domain: Some(domain), origin: None } if *domain == Domain::boolean(true))
        );
    }
}

#[test]
pub(crate) fn scalar_boolean_places_reject_old_reads_even_when_later_cells_match() {
    let (mut checker, _, _, mut snapshots) =
        capture("f:(a<boolean>,c<boolean>){x:=a;|c|x=true;|x|x;x=a;|c|x=true;|x|x}");
    let mut facts = snapshots
        .guards
        .iter()
        .filter(|(_, fact)| {
            snapshots.expressions[&fact.input.source.input]
                .value
                .as_ref()
                .unwrap()
                .version
                .is_none()
        })
        .collect::<Vec<_>>();
    facts.sort_by_key(|(_, fact)| fact.input.point);
    assert_eq!(facts.len(), 2);
    let old = facts[0].1.input.source.input;
    let Value::Known(first) = snapshots.rows[&Port::Normal(*facts[0].0)].after else {
        panic!()
    };
    let Value::Known(later) = snapshots.rows[&Port::Normal(*facts[1].0)].after else {
        panic!()
    };
    let local = snapshots.expressions[&old]
        .value
        .as_ref()
        .unwrap()
        .local
        .unwrap();
    assert_eq!(
        snapshots.states[first].get(local),
        snapshots.states[later].get(local)
    );
    assert_ne!(first, later);
    assert!(
        snapshots
            .boolean(old, later, true, &mut checker.flow, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
}

#[test]
pub(crate) fn scalar_boolean_places_require_local_evidence_and_reject_stale_cells() {
    for fault in 0..7 {
        let (mut checker, mut snapshots, edge, before, input, local) = setup();
        let scalar = snapshots
            .expressions
            .get_mut(&input)
            .unwrap()
            .value
            .as_mut()
            .unwrap();
        match fault {
            0 => scalar.local = None,
            1 => scalar.local = Some(usize::MAX),
            2 => {
                scalar.version = Some(Version {
                    owner: 1,
                    local,
                    definition: versions::Definition::Parameter(0),
                })
            }
            3 => scalar.origin = Some(Origin::Expression(1, input)),
            4 => scalar.control = true,
            5 => snapshots.expressions.get_mut(&input).unwrap().owner += 1,
            6 => {
                let Value::Known(id) = before else { panic!() };
                snapshots.states[id].cells.get_mut(&local).unwrap().data = Data::Partial;
            }
            _ => unreachable!(),
        }
        let len = snapshots.states.len();
        let room = snapshots.room();
        let result = snapshots.branch(edge, before, &mut checker.flow, Span::default());
        if fault == 0 {
            assert_eq!(result.unwrap(), before);
        } else {
            assert!(result.unwrap_err().message.contains("identity"), "{fault}");
        }
        assert_eq!(snapshots.states.len(), len);
        assert_eq!(snapshots.room(), room);
    }
}

#[test]
pub(crate) fn scalar_boolean_places_bound_merged_refinement_without_replacing_history() {
    let (mut checker, mut snapshots, edge, before, _, _) = setup();
    let start = checker.flow.work;
    let room = snapshots.room();
    let original = format!("{:?}{:?}", snapshots.rows, snapshots.expressions);
    let next = snapshots
        .branch(edge, before, &mut checker.flow, Span::default())
        .unwrap();
    assert_ne!(next, before);
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
        assert_eq!(
            format!("{:?}{:?}", snapshots.rows, snapshots.expressions),
            original
        );
    }
}
