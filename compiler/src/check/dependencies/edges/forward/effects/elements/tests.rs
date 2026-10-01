use super::{super::tests::checked, *};

#[test]
pub(crate) fn element_effects_retain_owned_view_temporary_and_nested_sources() {
    for source in [
        "xs:[1,2];p:&(xs[2])",
        "xs:=[1,2];p:&(xs[1])",
        "xs:[1,2];v:&xs;p:&(v[1])",
        "x:*(&([1,2][1]))",
        "r:{->inner:{->xs:[1,2]}};p:&(r.inner.xs[2])",
        "rows:[{->xs:[1]}];n:*(&(rows[1].xs[1]))",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert!(checker.locals.is_empty());
        assert!(!checker.elements.is_empty());
        for (&id, op) in &checker.elements {
            let access = op.access.unwrap();
            assert_eq!(
                reports.effects[&id],
                (
                    op.owner,
                    Effect::Element(Observed {
                        parent: op.parent,
                        source: op.source.clone(),
                        access,
                        control: false,
                        address: true,
                        acquired: true,
                        result: true,
                    })
                )
            );
            assert_eq!(op.edges[3].route, Route::Checked);
            assert!(!checker.indices.contains_key(&id));
            if op.source == ElementSource::View {
                assert_eq!(access.length, None);
            }
        }
    }
}

#[test]
pub(crate) fn element_effects_keep_address_acquisition_and_result_visits_independent() {
    let (mut checker, mut reports) = checked("r:{->inner:{->xs:[1]}};p:&(r.inner.xs[1])", false);
    let id = *checker.elements.first_key_value().unwrap().0;
    let address = Port::Address { point: id, step: 0 };
    for port in [address, Port::Operation(id), Port::Normal(id)] {
        reports.entries.get_mut(&0).unwrap().1.ports = vec![port, port];
        let effects = checker
            .operation_effects_limited(&reports, Span::default(), 1, 4, 0)
            .unwrap();
        assert_eq!(effects.len(), 1);
        let (_, Effect::Element(op)) = &effects[&id] else {
            panic!()
        };
        assert_eq!(op.address, port == address);
        assert_eq!(op.acquired, port == Port::Operation(id));
        assert_eq!(op.result, port == Port::Normal(id));
    }
}

#[test]
pub(crate) fn element_effects_retain_partial_addresses_before_stopped_positions() {
    for source in [
        "xs:[1];'out{p:&(xs[{'out.leave()}])}",
        "xs:[1];v:&xs;'out{p:&(v[{'out.leave()}])}",
        "'out{p:&([1][{'out.leave()}])}",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        let (&id, op) = checker.elements.first_key_value().unwrap();
        let (_, Effect::Element(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(observed.parent, op.parent);
        assert_eq!(observed.source, op.source);
        assert!(!observed.access.may_return);
        assert!(observed.address);
        assert!(!observed.acquired && !observed.result);
        assert!(!reports.index.operations.contains_key(&id));
    }
}

#[test]
pub(crate) fn element_effects_preserve_independent_owners_and_control() {
    let source = "flag:false;xs:[1];|flag|p:&(xs[1]);f:(items<&int32[2]>){p:&(items[1])}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    assert_eq!(checker.elements.len(), 2);
    assert!(checker.elements.values().any(|op| op.control));
    assert!(checker.elements.values().any(|op| op.owner != 0));
    for (&id, op) in &checker.elements {
        let (owner, Effect::Element(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert_eq!(observed.control, op.control);
        assert!(observed.address && observed.acquired && observed.result);
    }
}
