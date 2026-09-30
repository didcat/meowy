use super::{super::tests::checked, *};

#[test]
pub(crate) fn place_borrow_effects_exclude_stopped_successors_and_other_producers() {
    for source in [
        "n:1;'out{'out.leave();p:&n}",
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};n:1;stop();p:&n",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert_eq!(checker.place_borrows.len(), 1);
        for id in checker.place_borrows.keys() {
            assert!(!reports.effects.contains_key(id));
        }
    }
    for (source, count) in [
        ("n:1;p:&n;q:&*p", 1),
        ("n:=1;p:&!n;q:&!*p", 1),
        ("r:{->x:1};p:&r;q:&(p.x)", 1),
        ("xs:=[1];p:&!(xs[1])", 0),
        ("xs:[1];p:&(xs[1])", 1),
        ("p:&1", 0),
        ("<T>:{n:2;-><int32[n]>}", 0),
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert_eq!(checker.place_borrows.len(), count, "{source}");
        assert_eq!(
            reports
                .effects
                .values()
                .filter(|(_, op)| matches!(op, Effect::Borrow(_)))
                .count(),
            count
        );
        for id in checker
            .elements
            .keys()
            .chain(checker.exclusives.keys())
            .chain(checker.reborrow_ops.keys())
        {
            assert!(!matches!(
                reports.effects.get(id),
                Some((_, Effect::Borrow(_)))
            ));
        }
    }
    let source = "proof:@\"proof\";n:1;p:&n;q:proof.can_copy<uint8>()";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "B001");
    assert_eq!(checked(source, false).0.place_borrows.len(), 1);
}

#[test]
pub(crate) fn place_borrow_effects_preserve_mutability_conflicts_lifetimes_and_capabilities() {
    for (source, code) in [
        ("p:&missing", "E201"),
        ("r:{->x:1};p:&(r.missing)", "E201"),
        ("r:={->x:1};p:&!(r.x)", "E305"),
        ("n:=1;p:&!n;q:p;x:*p", "E301"),
        ("n:=1;p:&n;n=2;x:*p", "E302"),
        ("n:=1;p:&!n;q:&n;x:*p", "E302"),
        ("p:{n:1;->&n}", "E303"),
        ("r:={->x:1};p:&!r", "B001"),
        ("n:=1;p:&!n;q:&p", "B001"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn place_borrow_effects_reject_changed_canonical_alias_roots() {
    let source = "f:(b<boolean>)'out{|b|{'out->n:=1;p:&n};|!b|{'out->n:=2;p:&n}}";
    let (mut checker, reports) = checked(source, false);
    let op = checker.place_borrows.values().next().unwrap();
    checker.proofs.aliases.get_mut(&op.place.root).unwrap().root = reports.locals;
    let before = reports.effects.clone();
    assert!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(reports.effects, before);
}
