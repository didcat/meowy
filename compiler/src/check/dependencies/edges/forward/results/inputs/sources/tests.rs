use super::{super::super::tests::checked, *};

#[test]
pub(crate) fn candidate_sources_preserve_exact_projection_slots_and_independent_owners() {
    let source = "r:{->7;->z:9;->a:true};s:'out{{'out->((r))}};f<{n<int32>}>:(){v:{->n:1};->v}";
    let (checker, reports) = checked(source);
    let mut count = 0;
    let mut owners = BTreeSet::new();
    for (&(destination, _, _), &(owner, input)) in &reports.candidate_inputs {
        match input.projection {
            Projection::Value => assert_eq!(input.source, None),
            projection => {
                let slot = input.source.unwrap();
                assert_eq!(
                    reports.slot_uses[&Port::Emission(input.candidate.emission)],
                    (owner, slot)
                );
                assert_eq!(checker.bodies[&slot.block].owner, owner);
                assert_ne!(slot.block, destination);
                assert_eq!(
                    slot.index,
                    match projection {
                        Projection::Primary => 0,
                        Projection::Field(index) => index + 1,
                        _ => unreachable!(),
                    }
                );
                owners.insert(owner);
                count += 1;
            }
        }
    }
    assert_eq!(count, 5);
    assert_eq!(owners, BTreeSet::from([0, 1]));
}

#[test]
pub(crate) fn candidate_sources_require_existing_links_without_inventing_missing_slots() {
    let (mut checker, mut reports) = checked("r:{->7;->n:1};s:{->r}");
    let (&key, &(_, input)) = reports
        .candidate_inputs
        .iter()
        .find(|(_, (_, input))| input.projection == Projection::Primary)
        .unwrap();
    reports
        .slot_uses
        .remove(&Port::Emission(input.candidate.emission));
    let (inputs, _) = checker
        .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap();
    assert_eq!(inputs[&key].1.source, None);
    assert_eq!(
        inputs
            .values()
            .filter(|(_, input)| input.source.is_some())
            .count(),
        1
    );
    reports.slot_uses.clear();
    let (inputs, _) = checker
        .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap();
    assert!(inputs.values().all(|(_, input)| input.source.is_none()));
}

#[test]
pub(crate) fn candidate_sources_keep_direct_calls_reference_loads_and_field_values_separate() {
    for source in [
        "r:{->7;->n:1}",
        "f<{n<int32>}>:(){->n:1};s:{->f()}",
        "r:{->n:1};p:&r;s:{->*p}",
        "r:{->inner:{->n:1}};s:{->r.inner}",
    ] {
        let (_, reports) = checked(source);
        assert!(
            reports
                .candidate_inputs
                .values()
                .all(|(_, input)| input.source.is_none()),
            "{source}"
        );
    }
}
