use super::{super::tests::checked, *};
use std::collections::BTreeSet;

#[test]
pub(crate) fn temporary_effects_preserve_initializers_cells_and_reference_cell_identity() {
    for source in [
        "x:*(&7)",
        "x:*(&{->n:1})",
        "x:*(&[1,2])",
        "n:1;cell:&(&n)",
        "make<int32>:(){->7};x:*(&(make()))",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert!(checker.locals.is_empty());
        assert_eq!(checker.temporary_borrows.len(), 1);
        let (&id, op) = checker.temporary_borrows.first_key_value().unwrap();
        let cell = op.cell.unwrap();
        assert_eq!(
            reports.effects[&id],
            (
                op.owner,
                Effect::Temporary(Observed {
                    input: op.input,
                    local: cell.local,
                    statement: cell.statement,
                    control: false,
                    acquired: true,
                    result: true,
                })
            )
        );
        for borrow in checker.place_borrows.values() {
            assert_ne!(cell.local, borrow.storage);
        }
        for call in checker.invocations.values() {
            assert_eq!(call.point, op.input);
            assert_eq!(call.edges.last().unwrap().route, Route::Returned);
        }
    }
}

#[test]
pub(crate) fn temporary_effects_keep_acquisition_and_result_visits_independent() {
    let (mut checker, mut reports) = checked("x:*(&7)", false);
    let id = *checker.temporary_borrows.first_key_value().unwrap().0;
    for port in [Port::Operation(id), Port::Normal(id)] {
        reports.entries.get_mut(&0).unwrap().1.ports = vec![port, port];
        let effects = checker
            .operation_effects_limited(&reports, Span::default(), 1, 0, 0)
            .unwrap();
        assert_eq!(effects.len(), 1);
        let (_, Effect::Temporary(op)) = &effects[&id] else {
            panic!()
        };
        assert_eq!(op.acquired, port == Port::Operation(id));
        assert_eq!(op.result, port == Port::Normal(id));
    }
}

#[test]
pub(crate) fn temporary_effects_keep_nested_statement_lifetimes_owners_and_control() {
    let source = "flag:false;|flag|x:**(&(&7));f<int32>:(){->*(&3)};y:*(&{z:*(&2);->z})";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    assert_eq!(checker.temporary_borrows.len(), 5);
    assert!(checker.temporary_borrows.values().any(|op| op.control));
    assert!(checker.temporary_borrows.values().any(|op| op.owner != 0));
    let mut sites = BTreeMap::new();
    let mut locals = BTreeSet::new();
    for (&id, op) in &checker.temporary_borrows {
        let (owner, Effect::Temporary(observed)) = &reports.effects[&id] else {
            panic!()
        };
        let cell = op.cell.unwrap();
        assert_eq!(*owner, op.owner);
        assert_eq!(observed.control, op.control);
        assert_eq!(observed.statement, cell.statement);
        assert_eq!(checker.points[id].site, Some(cell.statement));
        assert_eq!(checker.proofs.temporaries[&observed.local], cell.statement);
        assert!(observed.acquired && observed.result);
        assert!(locals.insert(observed.local));
        *sites.entry(cell.statement).or_insert(0) += 1;
    }
    assert_eq!(sites.len(), 4);
    assert_eq!(sites.values().filter(|&&count| count == 2).count(), 1);
}
