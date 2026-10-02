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
pub(crate) fn result_sources_keep_exact_targets_and_checked_consumers() {
    let (checker, reports) = checked("row:{->n:1};copy:{->row}");
    let mut fields = 0;
    for (&id, (owner, observed)) in &reports.results {
        let body = &checker.bodies[&id];
        assert_eq!(*owner, body.owner);
        assert_eq!(observed.consumer, body.parent);
        let Layout::Slots(layout) = &body.layout else {
            panic!()
        };
        for (slot, sources) in layout.iter().zip(observed.slots.as_ref().unwrap()) {
            let Sources::Candidates(values) = sources else {
                panic!()
            };
            for value in values {
                assert_eq!(
                    checker.emission_sources[&value.emission],
                    (value.statement, value.target)
                );
                let target = &checker.emissions[&value.statement].targets[value.target];
                assert_eq!(target.block, id);
                assert_eq!(target.field, slot.field);
                fields += usize::from(slot.field.is_some());
            }
        }
    }
    assert_eq!(fields, 2);
}

#[test]
pub(crate) fn result_sources_preserve_unknown_slots_and_stopped_results() {
    let (checker, reports) = checked("f<int32><null>:(flag<boolean>){|flag|->1};row:{->n:=1;n=2}");
    let root = reports.entries[&1].0;
    assert_eq!(
        reports.results[&root].1,
        Observed {
            consumer: None,
            slots: None
        }
    );
    assert!(reports.results.values().any(|(_, row)| {
        row.slots
            .as_ref()
            .is_some_and(|slots| slots.contains(&Sources::Unknown))
    }));
    assert_eq!(checker.bodies[&root].layout, Layout::Unknown);
    let (_, reports) = checked("d:@\"debug\";value:{->1;d.panic(\"stop\")}");
    assert!(reports.results.is_empty());
}
