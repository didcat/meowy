use super::{super::super::super::tests::checked, *};

#[test]
pub(crate) fn field_narrowing_follows_only_unchanged_observed_field_wrappers() {
    let (mut checker, reports) = checked("r:{->n:1};x:r.n;f<null>:(){r:{->n:2};x:r.n}");
    let ids: Vec<_> = checker
        .narrowings
        .iter()
        .filter(|(_, op)| checker.fields.contains_key(&op.input))
        .map(|(&id, op)| (id, op.owner, op.input))
        .collect();
    assert_eq!(ids.len(), 2);
    let mut ctx = Lookup::new(&reports, MAX_EDGES);
    for (id, owner, field) in ids {
        let expected = Source {
            field,
            slot: reports.field_results[&Port::Normal(field)].1,
        };
        assert_eq!(
            checker
                .field_narrowing_source(&mut ctx, id, owner, Span::default(), 1)
                .unwrap(),
            Some(expected)
        );
        assert_eq!(
            checker
                .field_narrowing_source(&mut ctx, field, owner, Span::default(), 0)
                .unwrap(),
            Some(expected)
        );
        assert!(
            checker
                .field_narrowing_source(&mut ctx, id, owner, Span::default(), 0)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
}

#[test]
pub(crate) fn field_narrowing_keeps_changed_unobserved_and_other_producers_opaque() {
    for source in [
        "<U>:<int32><null>;r:{->n:1};x:r.n~<U>",
        "f<int32>:(){->1};x:f()",
        "r:{->n:1};p:&r;x:p.n",
    ] {
        let (mut checker, reports) = checked(source);
        let input = checker
            .operations
            .values()
            .filter_map(|op| op.input)
            .next_back()
            .unwrap();
        assert_eq!(
            checker
                .field_narrowing_source(
                    &mut Lookup::new(&reports, MAX_EDGES),
                    input,
                    0,
                    Span::default(),
                    MAX_GROUPS
                )
                .unwrap(),
            None,
            "{source}"
        );
    }
    let (mut checker, reports) = checked("r<{n<int32><null>}>:{->n:1};|r.n<int32>|x:r.n");
    let ids: Vec<_> = checker
        .narrowings
        .iter()
        .filter(|(_, op)| op.changed)
        .map(|(&id, op)| (id, op.owner))
        .collect();
    assert!(!ids.is_empty());
    for (id, owner) in ids {
        assert_eq!(
            checker
                .field_narrowing_source(
                    &mut Lookup::new(&reports, MAX_EDGES),
                    id,
                    owner,
                    Span::default(),
                    MAX_GROUPS
                )
                .unwrap(),
            None
        );
    }
    for missing in [false, true] {
        let (mut checker, mut reports) = checked("r:{->n:1};x:r.n");
        let (&id, _) = checker
            .narrowings
            .iter()
            .find(|(_, op)| checker.fields.contains_key(&op.input))
            .unwrap();
        if missing {
            reports.effects.remove(&id);
        } else {
            let (_, Effect::Narrowing(op)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            op.result = false;
        }
        assert_eq!(
            checker
                .field_narrowing_source(
                    &mut Lookup::new(&reports, MAX_EDGES),
                    id,
                    0,
                    Span::default(),
                    MAX_GROUPS
                )
                .unwrap(),
            None
        );
    }
}
