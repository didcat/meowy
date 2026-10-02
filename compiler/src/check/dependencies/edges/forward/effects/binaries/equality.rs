use super::{super::tests::checked, *};

#[test]
pub(crate) fn equality_effects_preserve_checked_whole_shape_categories_and_plain_result_routes() {
    for (source, class) in [
        (
            "a:{->7;->n:1};b:{->7;->n:2};x:a==b",
            Class::Record { fields: 1 },
        ),
        (
            "a<int32[3]>:[1];b<int32[3]>:[1,2];x:a!=b",
            Class::List { capacity: 3 },
        ),
        (
            "a<int32[0]>:[];b<int32[0]>:[];x:a==b",
            Class::List { capacity: 0 },
        ),
        (
            "n:1;m:2;x:(&n)==(&m)",
            Class::Reference(crate::hir::ReferenceMode::Shared),
        ),
        (
            "a<int32><null>:1;b<int32><null>:null;x:a!=b",
            Class::Union { members: 2 },
        ),
        (
            "m:@\"memory\";a:m.heap;b:m.heap;x:(&a)==(&b)",
            Class::Reference(crate::hir::ReferenceMode::Shared),
        ),
    ] {
        crate::compile(source).unwrap();
        let (mut checker, mut reports) = checked(source, false);
        let (&id, op) = checker.binaries.first_key_value().unwrap();
        assert_eq!(op.types.inputs, [class; 2]);
        assert!(op.plan.equality && !op.plan.checked);
        assert_eq!(op.plan.primary, [false; 2]);
        assert_eq!(op.edges.last().unwrap().route, Route::Next);
        let (_, Effect::Binary(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(observed.inputs, op.inputs);
        assert_eq!(observed.types, op.types);
        assert_eq!(observed.plan, op.plan);
        assert!(observed.operation && observed.result);
        assert_eq!(observed.projected, [false; 2]);
        for port in [Port::Operation(id), Port::Normal(id)] {
            reports.entries.get_mut(&0).unwrap().1.ports = vec![port, port];
            let effects = checker
                .operation_effects_limited(&reports, Span::default(), 1, 0, 0)
                .unwrap();
            let (_, Effect::Binary(observed)) = &effects[&id] else {
                panic!()
            };
            assert_eq!(observed.operation, port == Port::Operation(id));
            assert_eq!(observed.result, port == Port::Normal(id));
        }
    }
}

#[test]
pub(crate) fn equality_effects_preserve_partial_projections_and_stopped_successors() {
    let source = "f<never>:(r<{-><never>;tag<boolean>}>){->r==[1]}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    let (&id, op) = checker.binaries.first_key_value().unwrap();
    assert_eq!(op.types.inputs, [Class::Never, Class::List { capacity: 1 }]);
    let (_, Effect::Binary(observed)) = &reports.effects[&id] else {
        panic!()
    };
    assert_eq!(observed.projected, [true, false]);
    assert!(!observed.operation && !observed.result && !observed.plan.equality);
    assert!(
        !reports.entries[&op.owner]
            .1
            .ports
            .contains(&Port::Entry(op.inputs[1]))
    );
    let source = "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};a:[1];x:a==stop();y:a==a";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.binaries.len(), 2);
    assert!(
        checker
            .binaries
            .keys()
            .all(|id| !reports.effects.contains_key(id))
    );
}

#[test]
pub(crate) fn equality_effects_preserve_opaque_exclusions_and_ordinary_eligibility_errors() {
    let (mut checker, mut reports) = checked("1==2", false);
    let id = *checker.binaries.first_key_value().unwrap().0;
    checker.binaries.get_mut(&id).unwrap().types.inputs = [Class::Other; 2];
    reports.entries.get_mut(&0).unwrap().1.ports = vec![Port::Operation(id), Port::Normal(id)];
    let effects = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    assert_eq!(effects[&id].1, Effect::Unknown);
    let source = "a:=1;b:=2;r:&!a;s:&!b;x:r==s";
    let error = &crate::compile(source).unwrap_err()[0];
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("exclusive reference comparison"));
    let (checker, reports) = checked(source, false);
    let id = *checker.binaries.first_key_value().unwrap().0;
    assert_eq!(reports.effects[&id].1, Effect::Unknown);
    for source in [
        "m:@\"memory\";x:m.heap==m.heap",
        "m:@\"memory\";a:{->n:m.heap};b:a;x:a==b",
        "m:@\"memory\";a<m.Allocator[0]>:[];b<m.Allocator[0]>:[];x:a==b",
        "m:@\"memory\";a<m.Allocator><null>:null;b<m.Allocator><null>:null;x:a==b",
        "a:{->n:1};b:{->n:true};x:a==b",
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            "E222",
            "{source}"
        );
    }
}
