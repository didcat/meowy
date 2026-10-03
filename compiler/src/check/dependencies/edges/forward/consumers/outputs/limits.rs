use super::super::{limits::rejected, tests::checked, *};

#[test]
pub(crate) fn output_slots_keep_sparse_part_indices_and_independent_owners() {
    let source = "d:@\"debug\";r:{->7;->tag:true};s:((r));d.print(\"a{s}b{1}c{r}d{({->9;->tag:false})}\");f<null>:(p<{-><int32>;tag<boolean>}>){r:{->3;->tag:true};d.print(\"x{r}y{r}z{r}:{p}\")}";
    let (mut checker, reports) = checked(source);
    assert_eq!(reports.slot_uses.len(), 6);
    let mut steps = BTreeMap::new();
    for (&port, &(owner, slot)) in &reports.slot_uses {
        let Port::Projection { point, step } = port else {
            panic!()
        };
        let Effect::Output(op) = &reports.effects[&point].1 else {
            panic!()
        };
        assert!(op.parts[&step].projection);
        assert_eq!(slot.index, 0);
        assert_eq!(checker.bodies[&slot.block].owner, owner);
        let input = op.parts[&step].input.unwrap().point;
        let anchor = checker
            .grouped_consumer(&reports, input, owner, Span::default())
            .unwrap()
            .unwrap();
        assert_eq!(reports.consumers[&anchor], (owner, slot.block));
        steps.entry(owner).or_insert_with(Vec::new).push(step);
    }
    assert_eq!(steps[&0], [1, 5, 7]);
    assert_eq!(steps[&1], [1, 3, 5]);
}

#[test]
pub(crate) fn output_slots_require_projection_visits_and_keep_stopped_anchors() {
    for method in ["print", "panic"] {
        let source = format!("d:@\"debug\";r:{{->7;->tag:true}};d.{method}(\"a{{r}}b{{r}}\")");
        let (mut checker, mut reports) = checked(&source);
        let (&id, op) = checker.outputs.first_key_value().unwrap();
        let owner = op.owner;
        let prior = reports.slot_uses.clone();
        assert_eq!(prior.len(), 2);
        let port = Port::Projection { point: id, step: 3 };
        for state in 0..(if method == "panic" { 4 } else { 3 }) {
            let Effect::Output(op) = &mut reports.effects.get_mut(&id).unwrap().1 else {
                panic!()
            };
            op.prefix = state == 3;
            op.terminal = state == 2;
            if state < 2 {
                op.parts.retain(|part, _| *part == 3);
                let part = op.parts.get_mut(&3).unwrap();
                part.projection = state == 0;
                part.output = state == 1;
            } else {
                op.parts.clear();
            }
            reports.index.operations.remove(&id);
            if state == 2 {
                reports.index.operations.insert(id, owner);
            }
            let expected = if state == 0 {
                Uses::from([(port, prior[&port])])
            } else {
                Uses::new()
            };
            assert_eq!(
                checker.slot_uses(&reports, Span::default()).unwrap(),
                expected
            );
        }
        assert_eq!(reports.slot_uses, prior);
    }
    let (mut checker, mut reports) = checked("d:@\"debug\";r:{->7;->tag:true};d.print(r)");
    let (&id, _) = checker.outputs.first_key_value().unwrap();
    let prior = reports.slot_uses.clone();
    assert_eq!(prior.len(), 1);
    let op = checker.outputs.get_mut(&id).unwrap();
    let port = Port::Projection { point: id, step: 0 };
    let end = op.edges.iter().position(|edge| edge.to == port).unwrap() + 1;
    checker.output_edges -= op.edges.len() - end;
    op.edges.truncate(end);
    op.stopped = Some(0);
    let Effect::Output(op) = &mut reports.effects.get_mut(&id).unwrap().1 else {
        panic!()
    };
    op.stopped = Some(0);
    op.terminal = false;
    op.parts.get_mut(&0).unwrap().output = false;
    reports.index.operations.remove(&id);
    assert_eq!(checker.slot_uses(&reports, Span::default()).unwrap(), prior);
    let (_, reports) =
        checked("d:@\"debug\";f<null>:(r<{-><never>;tag<boolean>}>){d.print(\"a{r}tail{1}\")}");
    assert!(reports.slot_uses.is_empty());
}

#[test]
pub(crate) fn output_slots_discard_late_report_registration_and_edge_failures() {
    for fault in 0..18 {
        let source = "a:{->n:1}.n;d:@\"debug\";r:{->7;->tag:true};d.print(\"a{r}b{r}\")";
        let (mut checker, mut reports) = checked(source);
        let (&id, op) = checker.outputs.first_key_value().unwrap();
        let input = op.parts[3].unwrap().point;
        assert!(checker.fields.keys().all(|field| *field < id));
        let Effect::Output(op) = &mut reports.effects.get_mut(&id).unwrap().1 else {
            panic!()
        };
        match fault {
            0 => reports.effects.get_mut(&id).unwrap().0 += 1,
            1 => op.panic = !op.panic,
            2 => op.control = !op.control,
            3 => op.total += 1,
            4 => op.stopped = Some(1),
            5 => op.parts.get_mut(&3).unwrap().input.as_mut().unwrap().point = usize::MAX,
            6 => {
                op.parts
                    .get_mut(&3)
                    .unwrap()
                    .input
                    .as_mut()
                    .unwrap()
                    .primary = false
            }
            7 => {
                let part = op.parts.get_mut(&3).unwrap();
                part.projection = false;
                part.output = false;
            }
            8 => {
                reports.index.operations.remove(&id);
            }
            9 => {
                reports.index.operations.insert(id, usize::MAX);
            }
            10 => op.prefix = true,
            11 => {
                op.parts.insert(usize::MAX, op.parts[&3].clone());
            }
            12 => {
                checker.outputs.remove(&id);
            }
            13 => {
                checker
                    .outputs
                    .get_mut(&id)
                    .unwrap()
                    .edges
                    .last_mut()
                    .unwrap()
                    .route = Route::Next
            }
            14 => {
                let op = checker.outputs.get_mut(&id).unwrap();
                op.edges.push(op.edges[0]);
            }
            15 => checker.points[input].parent = None,
            16 => checker.points[input].block = None,
            17 => {
                let slot = reports.slot_uses[&Port::Projection { point: id, step: 3 }].1;
                reports.results.get_mut(&slot.block).unwrap().1.slots = None;
            }
            _ => unreachable!(),
        }
        let eligible = reports.eligible.clone();
        let initializers = reports.initializers.clone();
        let outputs = checker.outputs.clone();
        reports.parts = 0;
        rejected(&mut checker, &reports);
        assert_eq!(reports.eligible, eligible);
        assert_eq!(reports.initializers, initializers);
        assert_eq!(checker.outputs, outputs);
    }
}

#[test]
pub(crate) fn output_slots_deduplicate_visits_and_share_exact_map_and_work_limits() {
    let source = "a:{->n:1}.n;d:@\"debug\";r:{->7;->tag:true};d.print(\"a{r}b{r}c{r}\")";
    let (mut checker, mut reports) = checked(source);
    let prior = reports.slot_uses.clone();
    let effects = reports.effects.clone();
    let results = reports.results.clone();
    let initializers = reports.initializers.clone();
    let outputs = checker.outputs.clone();
    let counts = checker.edge_counts();
    assert_eq!(prior.len(), 4);
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.extend(walk.ports.clone());
    }
    reports.effects = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    assert_eq!(reports.effects, effects);
    let base = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len()
        + reports.eligible.len()
        + reports.initializers.len();
    let limit = base + prior.len();
    reports.parts = 0;
    let before = checker.flow.work;
    assert_eq!(
        checker
            .slot_uses_limited(&reports, Span::default(), limit)
            .unwrap(),
        prior
    );
    let work = checker.flow.work - before;
    for room in [limit - 1, base - 1] {
        assert!(
            checker
                .slot_uses_limited(&reports, Span::default(), room)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.slot_uses_limited(&reports, Span::default(), limit);
        assert_eq!(result.is_ok(), short == 0);
        if let Ok(actual) = result {
            assert_eq!(actual, prior);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        }
    }
    assert_eq!(reports.slot_uses, prior);
    assert_eq!(reports.effects, effects);
    assert_eq!(reports.results, results);
    assert_eq!(reports.initializers, initializers);
    assert_eq!(reports.parts, 0);
    assert_eq!(checker.outputs, outputs);
    assert_eq!(checker.edge_counts(), counts);
}
