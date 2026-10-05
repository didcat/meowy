use super::{super::tests::checked, *};
use crate::check::dependencies::{ScalarKind, SequenceSource};

#[test]
pub(crate) fn scalar_block_reports_reject_corrupt_producers_rows_and_routes_atomically() {
    for fault in 0..23 {
        let (mut checker, mut reports) =
            checked("a:{->1};s:{->a};r:{->n:0};unused:r.n;b:{->2};t:{->((b))}");
        let direct = reports
            .direct_sources
            .values()
            .rev()
            .find(|(_, direct)| direct.block.is_some())
            .unwrap()
            .1;
        let source = direct.block.unwrap();
        let (point, block) = (source.consumer, source.slot.block);
        match fault {
            0 => reports.consumers.get_mut(&point).unwrap().0 += 1,
            1 => reports.consumers.get_mut(&point).unwrap().1 = usize::MAX,
            2 => reports.results.get_mut(&block).unwrap().0 += 1,
            3 => reports.blocks.get_mut(&block).unwrap().0 += 1,
            4 => reports.blocks.get_mut(&block).unwrap().1.result = false,
            5 => reports.results.get_mut(&block).unwrap().1.consumer = None,
            6 => reports.results.get_mut(&block).unwrap().1.slots = None,
            7 => reports
                .results
                .get_mut(&block)
                .unwrap()
                .1
                .slots
                .as_mut()
                .unwrap()
                .clear(),
            8 => reports.blocks.get_mut(&block).unwrap().1.completion.normal = false,
            9 => reports.blocks.get_mut(&block).unwrap().1.span.end += 1,
            10 => checker.bodies.get_mut(&block).unwrap().owner += 1,
            11 => checker.bodies.get_mut(&block).unwrap().parent = None,
            12 => checker.points[point].complete = false,
            13 => checker.points[point].owner += 1,
            14 => {
                checker
                    .endpoints
                    .get_mut(&SequenceSource::Expr(point))
                    .unwrap()[1]
                    .route = Route::Next
            }
            15 => {
                checker
                    .endpoints
                    .get_mut(&SequenceSource::Block(block))
                    .unwrap()
                    .last_mut()
                    .unwrap()
                    .route = Route::Next
            }
            16 | 17 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&block).unwrap().layout
                else {
                    panic!()
                };
                if fault == 16 {
                    slots[0].mutable = true;
                } else {
                    slots[0].field = Some("wrong".into());
                }
            }
            18 => {
                checker.bodies.get_mut(&block).unwrap().completion.result =
                    Shape::Scalar(ScalarKind::Bool)
            }
            19 => {
                reports.results.remove(&block);
            }
            20 => {
                reports.blocks.remove(&block);
            }
            21 | 22 => {
                let point = if fault == 21 { direct.point } else { point };
                checker
                    .fields
                    .insert(point, checker.fields.first_key_value().unwrap().1.clone());
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        if fault < 21 {
            assert!(
                checker
                    .scalar_block_source(&reports, direct.point, 0, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity"),
                "{fault}"
            );
        }
        assert!(
            checker
                .direct_sources(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert!(
            checker
                .direct_walk_report(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn scalar_block_reports_require_results_without_inventing_normal_visits() {
    let (mut checker, mut reports) = checked("n:{->1};s:{->((n))}");
    let (&key, &(_, direct)) = reports
        .direct_sources
        .iter()
        .find(|(_, (_, direct))| direct.block.is_some())
        .unwrap();
    let source = direct.block.unwrap();
    reports.blocks.get_mut(&source.slot.block).unwrap().1.normal = false;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert_eq!(
        checker
            .scalar_block_source(&reports, direct.point, 0, Span::default())
            .unwrap(),
        Some(source)
    );
    assert_eq!(
        checker.direct_sources(&reports, Span::default()).unwrap().0,
        reports.direct_sources
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    reports.blocks.get_mut(&source.slot.block).unwrap().1.result = false;
    assert!(
        checker
            .scalar_block_source(&reports, direct.point, 0, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    reports.blocks.get_mut(&source.slot.block).unwrap().1.result = true;
    reports.consumers.remove(&source.consumer);
    assert_eq!(
        checker
            .scalar_block_source(&reports, direct.point, 0, Span::default())
            .unwrap(),
        None
    );
    assert!(
        checker
            .direct_walk_report(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    let sources = checker.direct_sources(&reports, Span::default()).unwrap().0;
    assert_eq!(sources[&key].1.block, None);
    assert_eq!(sources[&key].1.point, direct.point);
}

#[test]
pub(crate) fn scalar_block_reports_charge_exact_wrapper_and_terminal_work_without_payload() {
    let (mut checker, mut reports) = checked("n<int32>:(({->1}));s:{->((n~<int32>))}");
    let direct = reports
        .direct_sources
        .values()
        .find(|(_, direct)| direct.block.is_some())
        .unwrap()
        .1;
    reports.parts = 0;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let work = checker.flow.work;
    assert_eq!(
        checker
            .scalar_block_source(&reports, direct.point, 0, Span::default())
            .unwrap(),
        direct.block
    );
    let work = checker.flow.work - work;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.scalar_block_source(&reports, direct.point, 0, Span::default());
        if short == 0 {
            assert_eq!(result.unwrap(), direct.block);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
