use super::{super::tests::checked, *};

#[test]
pub(crate) fn temporary_effects_exclude_stopped_initializers_and_successors() {
    for source in [
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};x:&(stop());later:&7",
        "'out{x:&({'out.leave()});later:&7}",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, reports) = checked(source, false);
        assert_eq!(checker.temporary_borrows.len(), 2);
        assert!(
            checker
                .temporary_borrows
                .keys()
                .all(|id| !reports.effects.contains_key(id))
        );
        let (&id, op) = checker
            .temporary_borrows
            .iter()
            .find(|(_, op)| op.cell.is_none())
            .unwrap();
        let owner = op.owner;
        assert_eq!(
            op.edges,
            [Edge::new(
                Port::Entry(id),
                Port::Entry(op.input),
                Route::Next
            )]
        );
        for port in [Port::Operation(id), Port::Normal(id)] {
            assert!(
                checker
                    .validate_temporary_borrow(&reports, owner, port, Span::default())
                    .is_err()
            );
        }
    }
}

#[test]
pub(crate) fn temporary_effects_preserve_short_circuit_initializers_and_separate_producers() {
    for source in ["x:*(&(false&&true))", "x:*(&(false||true))"] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        let (&id, op) = checker.temporary_borrows.first_key_value().unwrap();
        assert!(matches!(reports.effects[&id].1, Effect::Temporary(_)));
        assert_eq!(checker.points[op.input].parent, Some(id));
        assert!(
            checker
                .points
                .iter()
                .any(|point| matches!(point.kind, PointKind::And | PointKind::Or))
        );
    }
    for source in [
        "x:*(&(({->n:1}).n))",
        "x:*(&([1][1]))",
        "n:1;p:&n;q:&*p",
        "<T>:{n:2;-><int32[n]>}",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert!(checker.temporary_borrows.is_empty());
        assert!(
            !reports
                .effects
                .values()
                .any(|(_, op)| matches!(op, Effect::Temporary(_)))
        );
    }
    let source = "p:@\"proof\";x:*(&7);q:p.can_copy<uint8>()";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "B001");
    assert_eq!(checked(source, false).0.temporary_borrows.len(), 1);
}

#[test]
pub(crate) fn temporary_effects_preserve_initializer_errors_and_statement_lifetimes() {
    for (source, code) in [
        ("p:&missing", "E201"),
        ("p:&(1/0)", "E107"),
        ("p:&1;x:*p", "E303"),
        ("n:1;p:&(&n);x:**p", "E303"),
        ("p:{->&7};x:*p", "E303"),
        ("n:=1;p:&(&!n)", "B001"),
        ("n:=1;p:&n;n=2;x:*(&(*p))", "E302"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}
