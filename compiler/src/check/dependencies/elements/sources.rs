use super::*;

pub(super) fn check(source: &str) -> (Checker, hir::Block) {
    let mut checker = Checker::new();
    let body = checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    (checker, body)
}

#[test]
pub(crate) fn element_place_sources_retain_nested_bounds_after_local_transfer() {
    let source = "r:{->a:0;->inner:{->xs:[1,2];->z:3}};p:&(r.inner.xs[1])";
    crate::compile(source).unwrap();
    let (mut checker, body) = check(source);
    let (&id, op) = checker.elements.first_key_value().unwrap();
    let Source::Place {
        place,
        storage,
        counts,
    } = &op.source
    else {
        panic!()
    };
    assert_eq!(place.fields, [1, 0]);
    assert_eq!(*storage, place.root);
    assert_eq!(counts, &[2, 2]);
    let before = op.clone();
    let program = hir::Program {
        body,
        functions: Vec::new(),
        locals: std::mem::take(&mut checker.locals),
    };
    checker.entry_reports(&program, Span::default()).unwrap();
    assert!(checker.locals.is_empty());
    assert_eq!(checker.elements[&id], before);
}

#[test]
pub(crate) fn element_place_sources_preserve_canonical_emitted_storage() {
    let source = "f:(flag<boolean>)'out{|flag|{'out->xs:=[1,2];p:&(xs[1])};|!flag|{'out->xs:=[3,4];p:&(xs[2])}}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let mut roots = Vec::new();
    let mut stores = Vec::new();
    for op in checker.elements.values() {
        let Source::Place {
            place,
            storage,
            counts,
        } = &op.source
        else {
            panic!()
        };
        assert!(counts.is_empty());
        assert_eq!(checker.proofs.aliases[&place.root].root, *storage);
        roots.push(place.root);
        stores.push(*storage);
    }
    assert_eq!(roots.len(), 2);
    assert_ne!(roots[0], roots[1]);
    assert_eq!(stores[0], stores[1]);
}

#[test]
pub(crate) fn element_place_sources_bound_paths_capture_work_and_atomic_publication() {
    let (mut checker, body) = check("r:{->a:0;->inner:{->xs:[1,2];->z:3}};p:&(r.inner.xs[1])");
    let hir::Stmt::Bind { value, .. } = &body.stmts[1] else {
        panic!()
    };
    let hir::ExprKind::ElementBorrow { value, .. } = &value.kind else {
        panic!()
    };
    let (&id, op) = checker.elements.first_key_value().unwrap();
    let op = op.clone();
    checker.elements.clear();
    checker.element_edges = 0;
    for fault in 0..4 {
        let mut value = value.as_ref().clone();
        let hir::ExprKind::Borrow(place) = &mut value.kind else {
            panic!()
        };
        match fault {
            0 => place.root = checker.locals.len(),
            1 => place.fields[0] = usize::MAX,
            2 => place.fields[1] = 1,
            3 => place.fields = vec![0; crate::list::MAX_WRITE_PATH + 1],
            _ => unreachable!(),
        }
        let error = checker
            .element_operation(id, op.parent, &value, op.access, op.span)
            .unwrap_err();
        assert!(
            error
                .message
                .contains(if fault == 3 { "budget" } else { "identity" })
        );
        assert!(checker.elements.is_empty());
        assert_eq!(checker.element_edges, 0);
    }
    let before = checker.flow.work;
    checker
        .element_operation(id, op.parent, value, op.access, op.span)
        .unwrap();
    let work = checker.flow.work - before;
    for spare in [0, 1] {
        checker.elements.clear();
        checker.element_edges = 0;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.element_operation(id, op.parent, value, op.access, op.span);
        assert_eq!(result.is_ok(), spare == 0);
        if result.is_ok() {
            assert_eq!(checker.elements[&id], op);
        } else {
            assert!(checker.elements.is_empty());
            assert_eq!(checker.element_edges, 0);
        }
    }
    checker.flow = crate::flow::Flow::default();
    checker
        .element_operation(id, op.parent, value, op.access, op.span)
        .unwrap();
    let Source::Place { counts, .. } = &mut checker.elements.get_mut(&id).unwrap().source else {
        panic!()
    };
    counts[0] += 1;
    let before = checker.elements.clone();
    let count = checker.element_edges;
    assert!(
        checker
            .element_operation(id, op.parent, value, op.access, op.span)
            .is_err()
    );
    assert_eq!(checker.elements, before);
    assert_eq!(checker.element_edges, count);
}
