use super::{super::tests::checked, *};

#[test]
pub(crate) fn exclusive_effects_keep_stopped_field_suffixes_and_other_producers_separate() {
    let source = "xs:=[{->inner:{->ys:=[1]}}];'out{p:&!(xs[{'out.leave()}].inner.ys[1])}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    let id = *checker.exclusives.first_key_value().unwrap().0;
    let (_, Effect::Exclusive(op)) = &reports.effects[&id] else {
        panic!()
    };
    assert_eq!(op.addresses, [true, false, false, false, false]);
    assert_eq!(op.reservations, [true, false, false, false]);
    assert!(!op.normal && !op.acquired && !op.result);
    let source = "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};xs:=[1];stop();p:&!(xs[1])";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.exclusives.len(), 1);
    assert!(
        checker
            .exclusives
            .keys()
            .all(|id| !reports.effects.contains_key(id))
    );
    for source in [
        "xs:[1];p:&(xs[1])",
        "n:=1;p:&!n;q:&!*p",
        "r:{->n:1};p:&r;q:&(p.n)",
        "<T>:{n:2;-><int32[n]>}",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert!(checker.exclusives.is_empty());
        assert!(
            !reports
                .effects
                .values()
                .any(|(_, op)| matches!(op, Effect::Exclusive(_)))
        );
    }
    let source = "proof:@\"proof\";xs:=[1];p:&!(xs[1]);q:proof.can_copy<uint8>()";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "B001");
    assert_eq!(checked(source, false).0.exclusives.len(), 1);
}

#[test]
pub(crate) fn exclusive_effects_preserve_alias_identity_and_original_length_snapshots() {
    let (mut checker, mut reports) = checked("f:(){r:{->xs:=[1];p:&!(xs[1]);*p=2}}", false);
    let (&id, op) = checker.exclusives.first_key_value().unwrap();
    let (owner, local) = (op.owner, op.place.root);
    reports.entries.get_mut(&owner).unwrap().1.ports = vec![Port::Address { point: id, step: 0 }];
    checker.proofs.aliases.get_mut(&local).unwrap().root = reports.locals;
    let before = reports.effects.clone();
    assert!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("exclusive-effect identity")
    );
    assert_eq!(reports.effects, before);
    let (mut checker, reports) = checked("xs:[{->n:=1}];p:&!(xs[1].n)", false);
    let (&id, op) = checker.exclusives.first_key_value().unwrap();
    let local = op.place.root;
    assert_eq!(op.access[0].length, Some(1));
    checker.lengths.get_mut(&local).unwrap().length = 0;
    let effects = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    assert_eq!(effects, reports.effects);
    let (_, Effect::Exclusive(op)) = &effects[&id] else {
        panic!()
    };
    assert_eq!(op.access[0].length, Some(1));
}

#[test]
pub(crate) fn exclusive_effects_preserve_index_permissions_reservations_moves_and_lifetimes() {
    for (source, code) in [
        ("p:&!(missing[false])", "E201"),
        ("xs:=[1];p:&!(xs[0])", "E101"),
        ("xs:=[1];p:&!(xs[false])", "E222"),
        ("xs:[1];p:&!(xs[1])", "E305"),
        ("xs:=[{->n:1}];p:&!(xs[1].n)", "E305"),
        ("xs:=[[1]];p:&!(xs[1][{xs=[[2]];->1}])", "E302"),
        ("xs:=[1];p:&!(xs[1]);xs[1]=2;x:*p", "E302"),
        ("p:{xs:=[1];->&!(xs[1])}", "E303"),
        ("xs:=[1];p:&!(xs[1]);copy:p;x:*p", "E301"),
        ("xs:=[[1]];p:&!(xs[1])", "B001"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}
