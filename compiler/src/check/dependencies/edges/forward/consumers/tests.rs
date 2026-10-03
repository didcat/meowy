use super::*;

pub(super) fn checked(source: &str) -> (Checker, Reports) {
    crate::compile(source).unwrap();
    let ast = crate::parser::parse(source).unwrap();
    let mut checker = Checker::new();
    let body = checker.block(&ast, None, None).unwrap();
    let program = hir::Program {
        body,
        functions: std::mem::take(&mut checker.functions)
            .into_iter()
            .flatten()
            .collect(),
        locals: std::mem::take(&mut checker.locals),
    };
    let reports = checker.entry_reports(&program, ast.span).unwrap();
    (checker, reports)
}

#[test]
pub(crate) fn field_slot_links_preserve_checked_order_and_independent_owners() {
    for source in [
        "x:{->z:1;->a:2}.z",
        "f<int32>:(){->{->z:3;->a:4}.a};x:{->n:{->n:1}.n}.n",
    ] {
        let (checker, reports) = checked(source);
        assert_eq!(reports.slot_uses.len(), checker.fields.len());
        for (&port, (owner, slot)) in &reports.slot_uses {
            let Port::Operation(id) = port else { panic!() };
            let field = &checker.fields[&id];
            assert_eq!(*owner, field.owner);
            assert_eq!(slot.index, field.index + 1);
            assert_eq!(reports.results[&slot.block].1.consumer, Some(field.input));
            let Layout::Slots(layout) = &checker.bodies[&slot.block].layout else {
                panic!()
            };
            assert!(layout[slot.index].field.is_some());
        }
    }
}

#[test]
pub(crate) fn field_slot_links_keep_unknown_values_and_indirect_inputs_separate() {
    let (_, reports) = checked("x:{->n:=1}.n;y:{->inner:{->n:2}}.inner");
    assert_eq!(reports.slot_uses.len(), 2);
    for (_, slot) in reports.slot_uses.values() {
        assert_eq!(
            reports.results[&slot.block].1.slots.as_ref().unwrap()[slot.index],
            super::super::results::Sources::Unknown
        );
    }
    for source in [
        "r:{->n:1};p:&r;v:p.n",
        "r:{->n:1};p:&r;v:(*p).n",
        "f<{n<int32>}>:(){->n:1};v:f().n",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
    let errors = crate::compile("d:@\"debug\";v:{d.panic(\"stop\");->n:1}.n").unwrap_err();
    assert_eq!(errors[0].code, "E201");
}

#[test]
pub(crate) fn narrowing_consumers_traverse_local_wrappers_without_initializer_joins() {
    for source in [
        "r:{->n:1};s<{n<int32>}>:((r));v:s.n",
        "f<int32>:(){r:{->n:1};s<{n<int32>}>:((r));->s.n}",
    ] {
        let (mut checker, reports) = checked(source);
        let root = checker
            .operations
            .values()
            .filter_map(|op| op.input)
            .find(|id| {
                checker
                    .coercions
                    .get(id)
                    .is_some_and(|op| checker.group_inputs.contains_key(&op.input))
            })
            .unwrap();
        let owner = checker.points[root].owner;
        let mut raw = root;
        for index in 0..6 {
            raw = if index == 5 {
                checker.narrowings[&raw].input
            } else if index % 2 == 0 {
                checker.coercions[&raw].input
            } else {
                checker.group_inputs[&raw].input
            };
        }
        assert!(matches!(reports.effects[&raw].1, Effect::Read { .. }));
        assert!(
            checker
                .grouped_consumer_limited(&reports, root, owner, Span::default(), 6)
                .unwrap()
                .is_none()
        );
        let error = checker
            .grouped_consumer_limited(&reports, root, owner, Span::default(), 5)
            .unwrap_err();
        assert!(error.message.contains("budget"));
        assert!(reports.slot_uses.is_empty());
    }
}

#[test]
pub(crate) fn local_read_consumer_boundaries() {
    for source in [
        "r:{->n:1};v:r.n",
        "r:{->n:1};v:((r)).n",
        "r:{->1;->tag:true};x:-r",
        "r:{->1;->tag:true};x:-((r))",
        "r<{n<int32>}>:(({->n:1}));v:r.n",
        "f<int32>:(){r<{n<int32>}>:(({->n:1}));->r.n}",
        "r<{n<int32>}>:(({->n:1}));copy:r.n;f<int32>:(flag<boolean>){|flag|x:2;n:3;->n}",
        "a:null;b:true;n:1;s:\"x\";xs:[1,2];r:{->n:1};u<int32><null>:n;f<int32>:(v<int32>){->v};copy:r.n",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
}
