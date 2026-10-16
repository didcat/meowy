use super::*;
use crate::check::dependencies::edges::forward::arguments::tests::context;

#[test]
pub(crate) fn argument_source_view_revalidates_late_field_block_and_dispatch_producers() {
    for kind in 0..3 {
        for fault in 0..3 {
            let (mut checker, mut reports) = context(
                "f:(a<int32>,b<int32>,c<int32>,d<int32>){};r:{->n:1};f(1,r.n,({->2}),3.{->$})",
            );
            let point = checker.invocations.first_key_value().unwrap().1.args[kind + 1];
            let direct = reports.argument_sources[&point].1;
            match kind {
                0 => {
                    let field = direct.source.unwrap().field;
                    match fault {
                        0 => {
                            reports
                                .field_results
                                .get_mut(&Port::Normal(field))
                                .unwrap()
                                .0 += 1
                        }
                        1 => {
                            reports
                                .field_results
                                .get_mut(&Port::Normal(field))
                                .unwrap()
                                .1
                                .index = usize::MAX
                        }
                        2 => reports.effects.get_mut(&field).unwrap().0 += 1,
                        _ => unreachable!(),
                    }
                }
                1 => {
                    let source = direct.block.unwrap();
                    match fault {
                        0 => reports.results.get_mut(&source.slot.block).unwrap().0 += 1,
                        1 => reports.blocks.get_mut(&source.slot.block).unwrap().1.result = false,
                        2 => checker.bodies.get_mut(&source.slot.block).unwrap().owner += 1,
                        _ => unreachable!(),
                    }
                }
                2 => {
                    let source = direct.dispatch.unwrap();
                    match fault {
                        0 => {
                            reports
                                .results
                                .get_mut(&source.slot.block)
                                .unwrap()
                                .1
                                .dispatch = None
                        }
                        1 => {
                            reports.results.remove(&source.slot.block);
                        }
                        2 => reports.effects.get_mut(&source.point).unwrap().0 += 1,
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            }
            let before = format!("{reports:?}{:?}{:?}", checker.bodies, checker.edge_counts());
            let error = checker
                .argument_source_view(&reports, Span::default(), MAX_EDGES)
                .unwrap_err();
            assert_eq!(error.code, "B001");
            assert!(
                error.message.contains("identity"),
                "{kind}/{fault}: {error:?}"
            );
            assert_eq!(
                format!("{reports:?}{:?}{:?}", checker.bodies, checker.edge_counts()),
                before
            );
        }
    }
}

#[test]
pub(crate) fn argument_source_view_requires_unknown_descriptors_after_source_evidence_is_omitted() {
    for field in [false, true] {
        let (mut checker, mut reports) =
            context("f:(a<int32>,b<int32>){};r:{->n:1};f(r.n,({->2}))");
        let call = checker.invocations.first_key_value().unwrap().1.clone();
        let point = call.args[usize::from(!field)];
        let direct = reports.argument_sources[&point].1;
        if field {
            reports
                .field_results
                .remove(&Port::Normal(direct.source.unwrap().field));
        } else {
            reports.consumers.remove(&direct.block.unwrap().consumer);
        }
        assert!(
            checker
                .argument_source_view(&reports, Span::default(), MAX_EDGES)
                .unwrap_err()
                .message
                .contains("identity")
        );
        reports.argument_sources.get_mut(&point).unwrap().1 = Direct {
            point,
            source: None,
            block: None,
            dispatch: None,
        };
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let (sources, _) = checker
            .argument_source_view(&reports, Span::default(), MAX_EDGES)
            .unwrap();
        assert!(std::ptr::eq(sources, &reports.argument_sources));
        assert_eq!(sources.len(), 2);
        let other = sources[&call.args[usize::from(field)]].1;
        assert!(other.source.is_some() || other.block.is_some());
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
