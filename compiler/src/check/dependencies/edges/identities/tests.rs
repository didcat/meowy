use super::*;
use crate::check::dependencies::SequenceSource;

pub(super) fn checked(source: &str) -> (Checker, crate::hir::Block) {
    let mut checker = Checker::new();
    let ast = crate::parser::parse(source).unwrap();
    let body = checker.block(&ast, None, None).unwrap();
    (checker, body)
}

#[test]
pub(crate) fn identity_endpoints_cross_resolved_foundation_and_function_aliases() {
    let source = "d:@\"debug\";console:d;say:console.print;die:d.panic;b:@\"bits\";flip:b.not;p:@\"proof\";query:p.can_copy;s:@\"strings\";copy:s.copy;f<int32>:(){->1};alias:(f);again:alias;x:2";
    crate::compile(source).unwrap();
    let (mut checker, body) = checked(source);
    let items = checker.sequences[&SequenceSource::Block(body.id)]
        .items
        .clone();
    for id in items[..items.len() - 1].iter().flatten() {
        assert_eq!(
            checker.endpoints[&SequenceSource::Stmt(*id)],
            [Edge::new(Port::Entry(*id), Port::Normal(*id), Route::Next)]
        );
    }
    let graph = checker.forward_index(Span::default()).unwrap();
    let walk = graph
        .walk(
            Port::BlockEntry(body.id),
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    assert!(
        walk.ports
            .contains(&Port::Operation(items.last().unwrap().unwrap()))
    );
    assert!(!walk.ports.contains(&Port::BlockEntry(
        checker.functions[0].as_ref().unwrap().body.id
    )));
}

#[test]
pub(crate) fn identity_endpoints_admit_self_and_reserved_forward_function_items() {
    for source in [
        "f<int32>:(n<int32>)'r{again:f;|n==0|{'r->0;'r.leave()};->again(n-1)};x:f(2)",
        "f<()->int32>;g<()->int32>;f<int32>:(){alias:g;->alias()};g<int32>:(){->1};x:f()",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, body) = checked(source);
        let function = checker.functions[0].as_ref().unwrap();
        let owner = function.id + 1;
        let block = function.body.id;
        let alias = checker.sequences[&SequenceSource::Block(block)].items[0].unwrap();
        assert_eq!(checker.points[alias].owner, owner);
        assert!(checker.endpoints.contains_key(&SequenceSource::Stmt(alias)));
        let graph = checker.forward_index(Span::default()).unwrap();
        let walk = graph
            .walk(Port::BlockEntry(block), &mut checker.flow, Span::default())
            .unwrap();
        assert!(walk.ports.contains(&Port::Normal(alias)));
        assert!(!walk.ports.contains(&Port::BlockEntry(body.id)));
    }
}

#[test]
pub(crate) fn identity_endpoints_cross_control_aliases_and_keep_forward_barriers() {
    for (source, reached) in [
        ("'out{finish:'out.leave;x:2}", true),
        ("f<()->int32>;f<int32>:(){->1};x:2", false),
    ] {
        crate::compile(source).unwrap();
        let (mut checker, body) = checked(source);
        let graph = checker.forward_index(Span::default()).unwrap();
        let walk = graph
            .walk(
                Port::BlockEntry(body.id),
                &mut checker.flow,
                Span::default(),
            )
            .unwrap();
        assert_eq!(
            checker
                .operations
                .keys()
                .any(|id| walk.ports.contains(&Port::Operation(*id))),
            reached,
            "{source}"
        );
    }
    let (checker, body) = checked("m:@\"memory\";h:m.heap;copy:h;x:1");
    for id in checker.sequences[&SequenceSource::Block(body.id)].items[1..]
        .iter()
        .flatten()
    {
        assert!(!checker.endpoints.contains_key(&SequenceSource::Stmt(*id)));
        assert!(checker.operations.contains_key(id));
    }
}

#[test]
pub(crate) fn identity_endpoints_keep_resolution_errors_before_publication() {
    for (source, code) in [
        ("d:@\"debug\";d:@\"bits\"", "E203"),
        ("alias:missing", "E201"),
        ("d:@\"debug\";alias:d.missing", "E201"),
        ("d:=@\"debug\"", "B001"),
        ("d<int32>:@\"debug\"", "B001"),
        ("f<int32>:(){->1};alias:=f", "B001"),
    ] {
        let mut checker = Checker::new();
        let ast = crate::parser::parse(source).unwrap();
        assert_eq!(
            checker.block(&ast, None, None).unwrap_err().code,
            code,
            "{source}"
        );
        for (id, point) in checker.points.iter().enumerate() {
            if point.kind == PointKind::Stmt && !point.complete {
                assert!(!checker.endpoints.contains_key(&SequenceSource::Stmt(id)));
            }
        }
    }
    for value in [Value::Pending(0), Value::Foundation(Item::Heap)] {
        assert_eq!(BindingIdentity::capture(&value), None);
    }
}

#[test]
pub(crate) fn identity_endpoints_validate_function_slots_points_and_conflicts() {
    let (mut checker, body) = checked("f<int32>:(){->1};alias:f");
    let id = checker.sequences[&SequenceSource::Block(body.id)].items[1].unwrap();
    let span = Span::default();
    let count = checker.endpoint_edges;
    let item = checker.functions[0].take();
    checker
        .identity_binding_endpoint(id, BindingIdentity::Function(0), span)
        .unwrap();
    checker.functions[0] = item;
    assert!(
        checker
            .identity_binding_endpoint(id, BindingIdentity::Function(usize::MAX), span)
            .is_err()
    );
    checker.functions[0].as_mut().unwrap().id = 1;
    assert!(
        checker
            .identity_binding_endpoint(id, BindingIdentity::Function(0), span)
            .is_err()
    );
    checker.functions[0].as_mut().unwrap().id = 0;
    let identity = BindingIdentity::Function(0);
    checker.points[id].kind = PointKind::Expr;
    assert!(
        checker
            .identity_binding_endpoint(id, identity, span)
            .is_err()
    );
    checker.points[id].kind = PointKind::Stmt;
    checker.points[id].complete = false;
    assert!(
        checker
            .identity_binding_endpoint(id, identity, span)
            .is_err()
    );
    checker.points[id].complete = true;
    checker.owner = 1;
    assert!(
        checker
            .identity_binding_endpoint(id, identity, span)
            .is_err()
    );
    checker.owner = 0;
    checker
        .endpoints
        .insert(SequenceSource::Stmt(id), Vec::new());
    assert!(
        checker
            .identity_binding_endpoint(id, identity, span)
            .is_err()
    );
    assert!(checker.endpoints[&SequenceSource::Stmt(id)].is_empty());
    assert_eq!(checker.endpoint_edges, count);
}

#[test]
pub(crate) fn identity_endpoints_bound_atomic_edge_and_work_publication() {
    let (mut checker, body) = checked("d:@\"debug\"");
    let id = checker.sequences[&SequenceSource::Block(body.id)].items[0].unwrap();
    let key = SequenceSource::Stmt(id);
    let identity = BindingIdentity::Foundation;
    let span = Span::default();
    let count = checker.endpoint_edges;
    checker
        .identity_binding_endpoint(id, identity, span)
        .unwrap();
    assert_eq!(checker.endpoint_edges, count);
    checker.endpoints.remove(&key);
    checker.endpoint_edges -= 1;
    let total: usize = checker.edge_counts().iter().map(|(_, size)| size).sum();
    checker.sequence_edges += MAX_EDGES - total;
    assert!(
        checker
            .identity_binding_endpoint(id, identity, span)
            .is_err()
    );
    assert!(!checker.endpoints.contains_key(&key));
    assert_eq!(checker.endpoint_edges, count - 1);
    checker.sequence_edges -= 1;
    let before = checker.flow.work;
    checker
        .identity_binding_endpoint(id, identity, span)
        .unwrap();
    let work = checker.flow.work - before;
    checker.endpoints.remove(&key);
    checker.endpoint_edges -= 1;
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work + 1;
    assert!(
        checker
            .identity_binding_endpoint(id, identity, span)
            .is_err()
    );
    assert!(!checker.endpoints.contains_key(&key));
    assert_eq!(checker.endpoint_edges, count - 1);
    checker.flow = crate::flow::Flow::new();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work;
    checker
        .identity_binding_endpoint(id, identity, span)
        .unwrap();
    assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
}
