use super::{super::tests::checked, *};

#[test]
pub(crate) fn element_effects_exclude_stopped_parents_and_other_borrow_producers() {
    let source =
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};xs:[1];p:&(stop()[missing]);later:&(xs[1])";
    crate::compile(source).unwrap();
    let (mut checker, reports) = checked(source, false);
    assert_eq!(checker.elements.len(), 2);
    assert!(
        checker
            .elements
            .keys()
            .all(|id| !reports.effects.contains_key(id))
    );
    let (&id, op) = checker
        .elements
        .iter()
        .find(|(_, op)| op.access.is_none())
        .unwrap();
    assert_eq!(op.source, ElementSource::Stopped);
    for port in [
        Port::Address { point: id, step: 0 },
        Port::Operation(id),
        Port::Normal(id),
    ] {
        assert!(
            checker
                .validate_element_borrow(&reports, 0, port, Span::default())
                .is_err()
        );
    }
    for source in [
        "xs:[1];x:xs[1]",
        "xs:=[1];p:&!(xs[1])",
        "r:{->n:1};p:&r;q:&(p.n)",
        "n:1;p:&n;q:&*p",
        "x:*(&7)",
        "<T>:{n:2;-><int32[n]>}",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert!(checker.elements.is_empty());
        assert!(
            !reports
                .effects
                .values()
                .any(|(_, op)| matches!(op, Effect::Element(_)))
        );
    }
    let source = "proof:@\"proof\";xs:[1];p:&(xs[1]);q:proof.can_copy<uint8>()";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "B001");
    assert_eq!(checked(source, false).0.elements.len(), 1);
}

#[test]
pub(crate) fn element_effects_preserve_receiver_index_loan_and_lifetime_errors() {
    for (source, code) in [
        ("p:&(missing[false])", "E201"),
        ("xs:[1];p:&(xs[missing])", "E201"),
        ("xs:[1];p:&(xs[0])", "E101"),
        ("xs:[1];p:&(xs[2])", "E101"),
        ("xs:[1];p:&(xs[false])", "E222"),
        ("p:&([1][1]);x:*p", "E303"),
        ("p:{xs:[1];->&(xs[1])}", "E303"),
        ("xs:=[1];p:&(xs[1]);xs[1]=2;x:*p", "E302"),
        ("n:1;xs:[&n];p:&(xs[1])", "B001"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}
