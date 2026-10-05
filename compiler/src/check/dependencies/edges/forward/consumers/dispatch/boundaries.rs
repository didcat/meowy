use super::{super::tests::checked, *};

#[test]
pub(crate) fn scalar_dispatch_sources_reject_corrupt_stored_descriptors_and_origins() {
    for fault in 0..15 {
        let (mut checker, mut reports) =
            checked("v:3.{->$};s:{->((v))};n:{->1};a:{->n};r:{->n:2};b:{->r.n}");
        let (&key, &(owner, direct)) = reports
            .direct_sources
            .iter()
            .find(|(_, (_, direct))| direct.dispatch.is_some())
            .unwrap();
        let source = direct.dispatch.unwrap();
        let field = reports
            .direct_sources
            .values()
            .find_map(|(_, direct)| direct.source)
            .unwrap();
        let block = reports
            .direct_sources
            .values()
            .find_map(|(_, direct)| direct.block)
            .unwrap();
        let (stored_owner, stored) = reports.direct_sources.get_mut(&key).unwrap();
        match fault {
            0 => stored.point = usize::MAX,
            1 => stored.dispatch.as_mut().unwrap().point = usize::MAX,
            2 => stored.dispatch.as_mut().unwrap().slot.block = usize::MAX,
            3 => stored.dispatch.as_mut().unwrap().slot.index = 1,
            4 => stored.dispatch = None,
            5 => stored.source = Some(field),
            6 => stored.block = Some(block),
            7 => *stored_owner += 1,
            8 => {
                reports
                    .results
                    .get_mut(&source.slot.block)
                    .unwrap()
                    .1
                    .dispatch = None
            }
            9 => {
                reports
                    .results
                    .get_mut(&source.slot.block)
                    .unwrap()
                    .1
                    .consumer = Some(source.point)
            }
            10 => {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source.point).unwrap()
                else {
                    panic!()
                };
                op.result = false;
            }
            11 => {
                reports.results.remove(&source.slot.block);
            }
            12 => {
                reports.direct_sources.remove(&key);
            }
            13 => {
                let (_, stored) = reports
                    .direct_sources
                    .values_mut()
                    .find(|(_, direct)| {
                        direct.source.is_none()
                            && direct.block.is_none()
                            && direct.dispatch.is_none()
                    })
                    .unwrap();
                stored.dispatch = Some(source);
            }
            14 => {
                reports
                    .direct_sources
                    .insert((key.0, key.1, usize::MAX), (owner, direct));
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .direct_graph(&reports, Span::default(), MAX_EDGES)
                .err()
                .unwrap()
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
pub(crate) fn scalar_dispatch_sources_bound_qualification_work_without_extra_payload() {
    let (mut checker, mut reports) = checked("n<int32>:((3.{->$})~<int32>);s:{->((n))}");
    let direct = reports
        .direct_sources
        .values()
        .find(|(_, direct)| direct.dispatch.is_some())
        .unwrap()
        .1;
    reports.parts = 0;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let work = checker.flow.work;
    assert_eq!(
        checker
            .scalar_dispatch_source(&reports, direct.point, 0, Span::default())
            .unwrap(),
        direct.dispatch
    );
    let work = checker.flow.work - work;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.scalar_dispatch_source(&reports, direct.point, 0, Span::default());
        if short == 0 {
            assert_eq!(result.unwrap(), direct.dispatch);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
