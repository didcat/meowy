use super::{super::tests::checked, *};

pub(super) fn rejected(checker: &mut Checker, reports: &Reports, input: PointId, owner: usize) {
    let prior = format!("{reports:?}");
    let groups = checker.group_inputs.clone();
    let regions = checker.region_edges.clone();
    let counts = checker.edge_counts();
    let error = checker
        .grouped_consumer(reports, input, owner, Span::default())
        .unwrap_err();
    assert!(error.message.contains("identity"), "{error:?}");
    let error = checker.slot_uses(reports, Span::default()).unwrap_err();
    assert!(error.message.contains("identity"), "{error:?}");
    assert_eq!(format!("{reports:?}"), prior);
    assert_eq!(checker.group_inputs, groups);
    assert_eq!(checker.region_edges, regions);
    assert_eq!(checker.edge_counts(), counts);
}

#[test]
pub(crate) fn grouped_consumers_reject_corrupt_markers_points_and_edges_atomically() {
    for fault in 0..28 {
        let (mut checker, mut reports) = checked("a:{->n:1}.n;v:(({->n:2})).n");
        let (&field, value) = checker.fields.last_key_value().unwrap();
        let (input, owner) = (value.input, value.owner);
        let child = checker.group_inputs[&input].input;
        let block = reports.slot_uses[&Port::Operation(field)].1.block;
        let anchor = reports.results[&block].1.consumer.unwrap();
        match fault {
            0 => checker.group_inputs.get_mut(&input).unwrap().owner += 1,
            1 => checker.group_inputs.get_mut(&input).unwrap().input = usize::MAX,
            2 => checker.group_inputs.get_mut(&input).unwrap().block = None,
            3 => checker.group_inputs.get_mut(&input).unwrap().span.end += 1,
            4 => checker.points[input].kind = PointKind::Stmt,
            5 => checker.points[input].complete = false,
            6 => checker.points[input].owner += 1,
            7 => checker.points[input].block = None,
            8 => checker.points[input].span.start = checker.points[input].span.end + 1,
            9 => checker.points[child].parent = None,
            10 => checker.points[child].complete = false,
            11 => checker.points[child].owner += 1,
            12 => checker.points[child].block = None,
            13 => checker.points[child].kind = PointKind::Stmt,
            14 => checker.points[child].span.start = checker.points[child].span.end + 1,
            15 => checker.points[child].span.start = checker.points[input].span.start - 1,
            16 => checker.points[child].span.end = checker.points[input].span.end + 1,
            17 => checker.region_edges.get_mut(&input).unwrap()[0].route = Route::Exit,
            18 => checker.region_edges.get_mut(&input).unwrap().swap(0, 1),
            19 => {
                checker.region_edges.remove(&input);
            }
            20 => checker.region_edges.get_mut(&input).unwrap()[0].from = Port::Normal(input),
            21 => checker.region_edges.get_mut(&input).unwrap()[1].to = Port::Entry(input),
            22 => {
                reports.consumers.insert(input, (owner, block));
            }
            23 => reports.consumers.get_mut(&anchor).unwrap().0 += 1,
            24 => checker.points[input].kind = PointKind::And,
            25 => checker.region_edges.get_mut(&input).unwrap()[1].route = Route::Result,
            26 => {
                let container = checker.group_inputs[&input].block.unwrap();
                checker.bodies.get_mut(&container).unwrap().owner += 1;
            }
            27 => {
                checker.group_inputs.get_mut(&input).unwrap().block = Some(usize::MAX);
                checker.points[input].block = Some(usize::MAX);
                checker.points[child].block = Some(usize::MAX);
            }
            _ => unreachable!(),
        }
        rejected(&mut checker, &reports, input, owner);
    }
}

#[test]
pub(crate) fn grouped_consumers_reject_self_and_consistent_two_group_cycles() {
    for pair in [false, true] {
        let (mut checker, reports) = checked("a:{->n:1}.n;v:(({->n:2})).n");
        let value = checker.fields.last_key_value().unwrap().1;
        let (input, owner) = (value.input, value.owner);
        let child = checker.group_inputs[&input].input;
        let span = checker.points[input].span;
        let chain = if pair {
            vec![(input, child), (child, input)]
        } else {
            vec![(input, input)]
        };
        for &(point, next) in &chain {
            checker.points[point].span = span;
            checker.points[next].parent = Some(point);
            let group = checker.group_inputs.get_mut(&point).unwrap();
            group.input = next;
            group.span = span;
            checker.region_edges.insert(
                point,
                [
                    Edge::new(Port::Entry(point), Port::Entry(next), Route::Next),
                    Edge::new(Port::Normal(next), Port::Normal(point), Route::Next),
                ],
            );
        }
        rejected(&mut checker, &reports, input, owner);
    }
}

#[test]
pub(crate) fn grouped_consumers_do_not_infer_groups_from_unmarked_region_edges() {
    let (mut checker, reports) = checked("v:(({->n:1})).n");
    let field = checker.fields.values().next().unwrap();
    let (input, owner) = (field.input, field.owner);
    checker.group_inputs.remove(&input);
    let groups = checker.group_inputs.clone();
    let regions = checker.region_edges.clone();
    let prior = format!("{reports:?}");
    let counts = checker.edge_counts();
    assert!(
        checker
            .grouped_consumer(&reports, input, owner, Span::default())
            .unwrap()
            .is_none()
    );
    assert!(
        checker
            .slot_uses(&reports, Span::default())
            .unwrap()
            .is_empty()
    );
    assert_eq!(format!("{reports:?}"), prior);
    assert_eq!(checker.group_inputs, groups);
    assert_eq!(checker.region_edges, regions);
    assert_eq!(checker.edge_counts(), counts);
}

#[test]
pub(crate) fn grouped_consumers_accept_logical_children_without_inventing_slots() {
    let (mut checker, reports) = checked("x:((false&&true));y:((true||false))");
    let groups = checker.group_inputs.clone();
    for kind in [PointKind::And, PointKind::Or] {
        assert!(
            groups
                .values()
                .any(|group| checker.points[group.input].kind == kind)
        );
    }
    let prior = format!("{reports:?}");
    let counts = checker.edge_counts();
    for (&point, group) in &groups {
        assert!(
            checker
                .grouped_consumer(&reports, point, group.owner, Span::default())
                .unwrap()
                .is_none()
        );
    }
    assert!(
        checker
            .slot_uses(&reports, Span::default())
            .unwrap()
            .is_empty()
    );
    assert_eq!(format!("{reports:?}"), prior);
    assert_eq!(checker.group_inputs, groups);
    assert_eq!(checker.edge_counts(), counts);
}
