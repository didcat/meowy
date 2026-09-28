use super::*;
use crate::check::dependencies::edges::{
    inventory::{FAMILIES, coverage::SOURCES},
    tests::check,
};

#[test]
pub(crate) fn forward_lookup_maps_every_family_entry_once_to_its_exact_source_and_route() {
    let mut covered = [false; FAMILIES];
    for source in SOURCES {
        let mut checker = check(source);
        let inventory = checker.edge_inventory(Span::default()).unwrap();
        let index = checker.forward_index(Span::default()).unwrap();
        assert_eq!(index.edges, inventory);
        let mut seen = vec![false; index.edges.len()];
        for (port, links) in &index.outgoing {
            for (positions, backedge) in [(&links.forward, false), (&links.backedges, true)] {
                for position in positions {
                    let (family, edge) = index.edges[*position];
                    assert!(!seen[*position]);
                    seen[*position] = true;
                    covered[family as usize] = true;
                    assert_eq!(edge.from, *port);
                    assert_eq!(edge.route == Route::Backedge, backedge);
                }
            }
        }
        assert!(seen.iter().all(|found| *found));
        assert!(index.outgoing.len() <= index.edges.len());
    }
    for (family, _) in Checker::new().edge_counts() {
        assert!(covered[family as usize], "{family:?}");
    }
}

#[test]
pub(crate) fn forward_lookup_preserves_function_owners_missing_ports_and_failure_atomicity() {
    let source = "f<int32>:(x<int32>){->x+1};g<int32>:(x<int32>){->f(x)};n:f(1);m:g(2)";
    let mut checker = check(source);
    let counts = checker.edge_counts();
    let before = checker.flow.work;
    let index = checker.forward_index(Span::default()).unwrap();
    let work = checker.flow.work - before;
    let mut owners = std::collections::BTreeSet::new();
    for (port, links) in &index.outgoing {
        let owner = checker.port_owner(*port, Span::default()).unwrap();
        owners.insert(owner);
        for position in &links.forward {
            let edge = index.edges[*position].1;
            assert_eq!(checker.port_owner(edge.to, Span::default()).unwrap(), owner);
        }
    }
    assert!(owners.contains(&0) && owners.len() >= 3);
    let destination = index
        .edges
        .iter()
        .map(|(_, edge)| edge.to)
        .find(|port| !index.outgoing.contains_key(port))
        .unwrap();
    assert!(!index.outgoing.contains_key(&destination));
    assert!(!index.outgoing.contains_key(&Port::Entry(usize::MAX)));
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work + 1;
    assert!(
        checker
            .forward_index(Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(checker.edge_counts(), counts);
    assert!(!index.outgoing.contains_key(&destination));
}

#[test]
pub(crate) fn forward_lookup_retains_full_capacity_positions_and_charges_empty_construction() {
    let port = Port::Entry(0);
    let edges = (0..MAX_EDGES)
        .map(|position| {
            (
                Family::Branch,
                Edge::new(
                    port,
                    Port::Normal(1),
                    if position % 2 == 0 {
                        Route::True
                    } else {
                        Route::Backedge
                    },
                ),
            )
        })
        .collect();
    let mut flow = crate::flow::Flow::new();
    let index = ForwardIndex::build(edges, &mut flow, Span::default()).unwrap();
    assert_eq!(index.outgoing.len(), 1);
    let links = &index.outgoing[&port];
    assert_eq!(links.forward.len() + links.backedges.len(), MAX_EDGES);
    assert_eq!(links.forward.last(), Some(&(MAX_EDGES - 2)));
    assert_eq!(links.backedges.last(), Some(&(MAX_EDGES - 1)));
    let mut flow = crate::flow::Flow::new();
    let empty = ForwardIndex::build(Vec::new(), &mut flow, Span::default()).unwrap();
    assert!(empty.edges.is_empty() && empty.outgoing.is_empty());
    assert_eq!(flow.work, 1);
    flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        ForwardIndex::build(Vec::new(), &mut flow, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
}
