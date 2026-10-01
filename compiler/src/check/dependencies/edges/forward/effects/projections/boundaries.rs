use super::{super::tests::checked, *};

#[test]
pub(crate) fn projection_effects_exclude_stopped_parents_successors_and_other_producers() {
    let source = "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};r:{->n:1};p:&r;q:&(stop().missing);later:&(p.n)";
    crate::compile(source).unwrap();
    let (mut checker, reports) = checked(source, false);
    assert_eq!(checker.projections.len(), 2);
    assert!(
        checker
            .projections
            .keys()
            .all(|id| !reports.effects.contains_key(id))
    );
    let (&id, op) = checker
        .projections
        .iter()
        .find(|(_, op)| op.site.is_none())
        .unwrap();
    assert!(op.steps.is_empty() && op.mode.is_none());
    for port in [
        Port::Projection { point: id, step: 0 },
        Port::Conversion { point: id, part: 0 },
        Port::Operation(id),
        Port::Normal(id),
    ] {
        assert!(
            checker
                .validate_borrow_projection(&reports, 0, port, Span::default())
                .is_err()
        );
    }
    for source in [
        "n:1;p:&n;q:&*p",
        "xs:[1];p:&(xs[1])",
        "xs:=[1];p:&!(xs[1])",
        "x:*(&7)",
        "r:{->n:1};p:&(r.n)",
        "<T>:{n:2;-><int32[n]>}",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert!(checker.projections.is_empty());
        assert!(
            !reports
                .effects
                .values()
                .any(|(_, op)| matches!(op, Effect::Projection(_)))
        );
    }
    let source = "proof:@\"proof\";r:{->n:1};p:&r;q:&(p.n);answer:proof.can_copy<uint8>()";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "B001");
    assert_eq!(checked(source, false).0.projections.len(), 1);
}

#[test]
pub(crate) fn projection_effects_preserve_field_guard_loan_and_temporary_errors() {
    for (source, code) in [
        ("r:{->n:1};p:&r;q:&(p.missing)", "E201"),
        ("q:&(missing.n)", "E201"),
        ("p:&({->n:1}.n);x:*p", "E303"),
        ("r:={->n:1};h:{->p:&r};q:&(h.p.n);r={->n:2};x:*q", "E302"),
        (
            "<R>:<{n<int32>}>;r<R>:{->n:1};h:{->p<&R><null>:&r};q:&(h.p.n)",
            "B001",
        ),
        (
            "<R>:<{n<int32>}>;r<R>:{->n:1};h:{->p<&R><null>:=&r};|h.p<&R>|{h.p=null;q:&((h.p~<&R>).n)}",
            "E208",
        ),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}
