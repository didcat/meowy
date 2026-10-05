use super::*;
use crate::check::dependencies::edges::forward::results::tests::checked;

#[test]
pub(crate) fn dispatch_sources_preserve_exact_emissions_layouts_and_independent_owners() {
    let (mut checker, reports) = checked(
        "v:3.{->$};empty:3.{};row:3.{->n:=$};flag:=false;maybe:flag.{|$|->1};f<int32>:(){->4.{->$}}",
    );
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let mut owners = std::collections::BTreeSet::new();
    let mut kinds = [false; 3];
    for (&block, &(owner, ref row)) in &reports.results {
        let Some(id) = row.dispatch else {
            continue;
        };
        owners.insert(owner);
        assert_eq!(row.consumer, None);
        assert_eq!(checker.dispatch_ops[&id].block, block);
        assert!(!reports.blocks.contains_key(&block));
        assert!(!reports.consumers.contains_key(&id));
        assert!(
            !reports
                .initializers
                .contains_key(&checker.dispatch_ops[&id].local)
        );
        checker
            .validate_result_report(&reports, block, owner, row, Span::default())
            .unwrap();
        let Some(slots) = &row.slots else {
            kinds[0] = true;
            continue;
        };
        for source in slots {
            match source {
                Sources::Unknown => kinds[1] = true,
                Sources::Candidates(values) => {
                    kinds[2] |= values.is_empty();
                    for candidate in values {
                        assert_eq!(
                            checker.emission_sources[&candidate.emission],
                            (candidate.statement, candidate.target)
                        );
                        assert_eq!(
                            checker.emissions[&candidate.statement].targets[candidate.target].block,
                            block
                        );
                    }
                }
            }
        }
    }
    assert_eq!(owners, std::collections::BTreeSet::from([0, 1]));
    assert_eq!(kinds, [true; 3]);
    assert_eq!(
        checker.result_sources(&reports, Span::default()).unwrap().0,
        reports.results
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn dispatch_sources_require_results_and_keep_target_visits_independent() {
    let (mut checker, mut reports) = checked("v:3.{->n:$}");
    let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
    let block = op.block;
    let expected = reports.results.clone();
    let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    op.initialized = false;
    assert_eq!(
        checker.result_sources(&reports, Span::default()).unwrap().0,
        expected
    );
    for (_, effect) in reports.effects.values_mut() {
        if let Effect::Emission(op) = effect {
            op.initialized.fill(false);
        }
    }
    let rows = checker.result_sources(&reports, Span::default()).unwrap().0;
    assert!(
        rows[&block]
            .1
            .slots
            .as_ref()
            .unwrap()
            .iter()
            .all(|source| *source == Sources::Candidates(vec![]))
    );
    let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    op.result = false;
    op.initialized = true;
    assert!(
        !checker
            .result_sources(&reports, Span::default())
            .unwrap()
            .0
            .contains_key(&block)
    );
    for tail in ["stop().{}", "3.{stop()}"] {
        let (_, reports) = checked(&format!(
            "d:@\"debug\";stop<never>:(){{d.panic(\"stop\")}};v:{tail}"
        ));
        assert!(
            reports
                .results
                .values()
                .all(|(_, row)| row.dispatch.is_none())
        );
    }
}
