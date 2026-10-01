use super::{super::tests::checked, *};

#[test]
pub(crate) fn reborrow_effects_exclude_stopped_parents_and_later_acquisitions() {
    for binding in ["q:&*(stop())", "q:&!*(stop())", "q<&int32>:stop()"] {
        let source = format!(
            "d:@\"debug\";stop<never>:(){{d.panic(\"stop\")}};n:1;p:&n;{binding};later:&*p"
        );
        crate::compile(&source).unwrap();
        let (mut checker, reports) = checked(&source, false);
        assert_eq!(checker.reborrow_ops.len(), 2);
        assert!(
            checker
                .reborrow_ops
                .keys()
                .all(|id| !reports.effects.contains_key(id))
        );
        let (&id, op) = checker
            .reborrow_ops
            .iter()
            .find(|(_, op)| op.site.is_none())
            .unwrap();
        assert!(op.parent_mode.is_none());
        assert_eq!(
            op.edges,
            [Edge::new(
                Port::Entry(id),
                Port::Entry(op.parent),
                Route::Next
            )]
        );
        for port in [Port::Operation(id), Port::Normal(id)] {
            assert!(
                checker
                    .validate_reborrow(&reports, 0, port, Span::default())
                    .is_err()
            );
        }
    }
}

#[test]
pub(crate) fn reborrow_effects_keep_never_referents_site_gaps_and_other_producers() {
    let source = "f:(p<&never>){q:&*p}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    let id = *checker.reborrow_ops.first_key_value().unwrap().0;
    let (_, Effect::Reborrow(op)) = &reports.effects[&id] else {
        panic!()
    };
    assert_eq!(op.mode, ReferenceMode::Shared);
    assert!(op.acquired && op.result);
    assert!(checker.derefs.is_empty());
    let (checker, reports) = checked("r:{->n:1};p:&r;field:&(p.n);q:&*p", false);
    assert_eq!(checker.reborrow_ops.len(), 1);
    let (&id, op) = checker.reborrow_ops.first_key_value().unwrap();
    assert_eq!(op.site, Some(1));
    let (_, Effect::Reborrow(observed)) = &reports.effects[&id] else {
        panic!()
    };
    assert_eq!(observed.site, 1);
    for source in [
        "r:{->n:1};p:&r;q:&(p.n)",
        "xs:=[1];p:&!(xs[1])",
        "n:1;p<&int32>:&n",
        "x:*(&7)",
        "<T>:{n:2;-><int32[n]>}",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert!(checker.reborrow_ops.is_empty());
        assert!(
            !reports
                .effects
                .values()
                .any(|(_, op)| matches!(op, Effect::Reborrow(_)))
        );
    }
    let source = "proof:@\"proof\";n:1;p:&n;r:&*p;q:proof.can_copy<uint8>()";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "B001");
    assert_eq!(checked(source, false).0.reborrow_ops.len(), 1);
}

#[test]
pub(crate) fn reborrow_effects_preserve_permissions_moves_suspension_and_lifetimes() {
    for (source, code) in [
        ("q:&!*missing", "E201"),
        ("n:1;p:&n;q:&!*p", "E305"),
        ("n:=1;p:&!n;copy:p;q:&!*p", "E301"),
        ("n:=1;p:&!n;q:&!*p;x:*p;y:*q", "E302"),
        ("n:=1;p:&!n;q:&*p;*p=2;x:*q", "E302"),
        ("n:=1;p:&!n;q<&int32>:p;*p=2;x:*q", "E302"),
        ("q:{n:=1;p:&!n;->&!*p}", "E303"),
        ("q:&*(&(1+2));x:*q", "E303"),
        ("n:=1;p:&!n;q<&boolean>:p", "E207"),
        ("f:(p<&!never>){q:&!*p}", "B001"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}
