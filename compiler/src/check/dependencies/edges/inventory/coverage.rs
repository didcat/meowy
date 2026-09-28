use super::*;
use crate::check::{SequenceSource, dependencies::edges::tests::check};

pub(crate) const SOURCES: &[&str] = &[
    "m:@\"memory\";d:@\"debug\";x:1;p:&x;q:&*p;y:*q;z:-y;h:m.heap;r:{->n:2};v:r.n~<int32>;d.print(z)",
    "xs<int32[2]>:=[1];xs[1]=2;v:xs[1];ys:xs.add(2);count:ys.size();p:&(xs[1]);copy:*p",
    "x:=1;p:&!x;*p=2",
    "xs:=[1];p:&!(xs[1]);*p=2",
    "r:{->n:1};h:{->p:&r};q:&(h.p.n);copy:*q",
    "x:*(&1);y:1.{->$+1}",
    "n:=0;'loop{|n==1|'loop.leave();n=n+1;|n<2|'loop.restart()};f<int32>:(n<int32>){->n+1};x:f(2)",
];

#[test]
pub(crate) fn edge_inventory_covers_every_stored_family_from_checked_source() {
    let mut covered = [false; FAMILIES];
    for source in SOURCES {
        crate::compile(source).unwrap();
        let mut checker = check(source);
        let counts = checker.edge_counts();
        let inventory = checker.edge_inventory(Span::default()).unwrap();
        let mut actual = [0usize; FAMILIES];
        for (family, _) in inventory {
            covered[family as usize] = true;
            actual[family as usize] += 1;
        }
        for (family, count) in counts {
            assert_eq!(actual[family as usize], count, "{family:?}: {source}");
        }
    }
    for (family, _) in Checker::new().edge_counts() {
        assert!(covered[family as usize], "uncovered {family:?}");
    }
}

#[test]
pub(crate) fn edge_inventory_preserves_mixed_function_owners_without_inventing_links() {
    let source = "f<int32>:(v<int32>){->v+1};g<int32>:(v<int32>){->f(v)};x:f(1);y:g(2)";
    let mut checker = check(source);
    let calls: Vec<_> = checker
        .invocations
        .values()
        .map(|call| (call.owner, call.point, call.edges.clone()))
        .collect();
    assert!(calls.iter().any(|(owner, _, _)| *owner == 0));
    assert!(calls.iter().any(|(owner, _, _)| *owner != 0));
    let inventory = checker.edge_inventory(Span::default()).unwrap();
    for (owner, point, edges) in calls {
        assert_eq!(checker.points[point].owner, owner);
        for edge in edges {
            assert!(inventory.contains(&(Family::Invocation, edge)));
        }
    }
    let call_count = inventory
        .iter()
        .filter(|(family, _)| *family == Family::Invocation)
        .count();
    assert_eq!(call_count, checker.invocation_edges);
}

#[test]
pub(crate) fn edge_inventory_enforces_actual_capacity_and_charges_empty_rows() {
    let mut checker = Checker::new();
    let edge = Edge::new(Port::Entry(0), Port::Normal(0), Route::Next);
    let key = SequenceSource::Expr(0);
    checker.endpoints.insert(key, vec![edge; MAX_EDGES]);
    checker.endpoint_edges = MAX_EDGES;
    let inventory = checker.edge_inventory(Span::default()).unwrap();
    assert_eq!(inventory.len(), MAX_EDGES);
    assert!(
        inventory
            .iter()
            .all(|entry| *entry == (Family::Endpoint, edge))
    );
    drop(inventory);
    checker.endpoints.get_mut(&key).unwrap().push(edge);
    assert!(
        checker
            .edge_inventory(Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(checker.endpoints[&key].len(), MAX_EDGES + 1);
    assert_eq!(checker.endpoint_edges, MAX_EDGES);
    let mut checker = Checker::new();
    checker
        .endpoints
        .insert(SequenceSource::Expr(0), Vec::new());
    checker
        .endpoints
        .insert(SequenceSource::Expr(1), Vec::new());
    checker.flow.work = crate::flow::MAX_PROOF_WORK - FAMILIES * 2 - 1;
    assert!(
        checker
            .edge_inventory(Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.flow.exceeded());
    assert_eq!(checker.endpoints.len(), 2);
    assert_eq!(checker.endpoint_edges, 0);
}

#[test]
pub(crate) fn edge_inventory_audit_keeps_ordinary_errors_and_pending_proof_gates() {
    for (source, code) in [
        ("x:boolean", "E201"),
        ("x:=1;p:&x;x=2;y:*p", "E302"),
        ("p:@\"proof\";q:p.can_copy<uint8>()", "B001"),
        (
            "p:@\"proof\";q:p.can_copy<uint8>();x:=1;r:&x;x=2;y:*r",
            "E302",
        ),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}
