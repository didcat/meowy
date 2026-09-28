use super::*;
use crate::check::dependencies::edges::tests::check;

#[test]
pub(crate) fn port_anchors_resolve_checked_points_bodies_emissions_and_restarts() {
    let source = "r:{->n:1};f<int32>:(v<int32>){->v};flag:false;'outer{'inner{|flag|'outer.restart();'inner.leave()}}";
    crate::compile(source).unwrap();
    let mut checker = check(source);
    let edges = checker.edge_inventory(Span::default()).unwrap();
    checker
        .validate_edge_ports(&edges, Span::default())
        .unwrap();
    let owners: Vec<_> = edges
        .iter()
        .map(|(_, edge)| checker.port_owner(edge.from, Span::default()).unwrap())
        .collect();
    assert!(owners.contains(&0) && owners.iter().any(|owner| *owner != 0));
    let (id, point) = checker
        .points
        .iter()
        .enumerate()
        .find(|(_, point)| point.complete)
        .unwrap();
    let owner = point.owner;
    assert_eq!(
        checker
            .port_owner(Port::Normal(id), Span::default())
            .unwrap(),
        owner
    );
}

#[test]
pub(crate) fn port_anchors_reject_missing_or_incomplete_points_and_special_targets() {
    let mut checker = check("r:{->n:1};flag:false;'loop{|flag|'loop.restart()}");
    for port in [
        Port::Entry(usize::MAX),
        Port::BlockResult(usize::MAX),
        Port::Emission(usize::MAX),
        Port::Restart {
            target: 0,
            site: usize::MAX,
        },
    ] {
        assert!(
            checker
                .port_owner(port, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
    checker.points[0].complete = false;
    assert!(
        checker
            .port_owner(Port::Normal(0), Span::default())
            .is_err()
    );
    checker.points[0].complete = true;
    let (&emit, &(source, index)) = checker.emission_sources.first_key_value().unwrap();
    checker.emission_sources.insert(emit, (source, index + 100));
    assert!(
        checker
            .port_owner(Port::Emission(emit), Span::default())
            .is_err()
    );
    let (&site, input) = checker.restart_inputs.first_key_value().unwrap();
    let target = input.target;
    assert!(
        checker
            .port_owner(
                Port::Restart {
                    site,
                    target: target + 100
                },
                Span::default()
            )
            .is_err()
    );
    checker.restart_inputs.get_mut(&site).unwrap().point = None;
    assert!(
        checker
            .port_owner(Port::Restart { site, target }, Span::default())
            .is_err()
    );
}

#[test]
pub(crate) fn port_anchor_validation_is_bounded_and_keeps_normal_flow_unknown() {
    let mut checker = check("f<never>:(p<never>){->p}");
    let (&id, read) = checker.local_reads.first_key_value().unwrap();
    assert!(!read.normal);
    let owner = read.owner;
    assert_eq!(
        checker
            .port_owner(Port::Normal(id), Span::default())
            .unwrap(),
        owner
    );
    let counts = checker.edge_counts();
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .port_owner(Port::Normal(id), Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(checker.edge_counts(), counts);
}
