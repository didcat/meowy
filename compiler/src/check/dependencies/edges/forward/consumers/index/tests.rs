use super::*;
use crate::check::dependencies::SequenceSource;

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
pub(crate) fn result_consumers_preserve_exact_owners_roots_and_unknown_layouts() {
    let (mut checker, reports) = checked(
        "f<int32><null>:(flag<boolean>){|flag|->1};row:{->f(true)};g<int32>:(){n:{->2};->n}",
    );
    let expected = reports
        .results
        .iter()
        .filter_map(|(&id, (owner, row))| row.consumer.map(|point| (point, (*owner, id))))
        .collect::<Index>();
    assert_eq!(reports.consumers, expected);
    assert_eq!(expected.len(), 2);
    assert!(expected.values().any(|(owner, _)| *owner == 0));
    assert!(expected.values().any(|(owner, _)| *owner == 2));
    assert!(
        expected
            .values()
            .any(|(_, id)| reports.results[id].1.slots.is_none())
    );
    for (id, _) in reports.entries.values() {
        assert_eq!(reports.results[id].1.consumer, None);
        assert!(!expected.values().any(|(_, block)| block == id));
    }
    assert_eq!(
        checker.result_consumers(&reports, Span::default()).unwrap(),
        expected
    );
}

#[test]
pub(crate) fn result_consumers_reject_corrupt_rows_and_endpoints_atomically() {
    for fault in 0..12 {
        let (mut checker, mut reports) = checked("a:{->1};b:{->2}");
        let (&point, &(owner, id)) = reports.consumers.last_key_value().unwrap();
        let other = reports.consumers.first_key_value().unwrap().0;
        match fault {
            0 => reports.results.get_mut(&id).unwrap().0 += 1,
            1 => reports.blocks.get_mut(&id).unwrap().0 += 1,
            2 => reports.blocks.get_mut(&id).unwrap().1.result = false,
            3 => reports.blocks.get_mut(&id).unwrap().1.span.end += 1,
            4 => reports.blocks.get_mut(&id).unwrap().1.completion.normal = false,
            5 => reports.results.get_mut(&id).unwrap().1.consumer = None,
            6 => reports.results.get_mut(&id).unwrap().1.consumer = Some(*other),
            7 => reports.results.get_mut(&id).unwrap().1.slots = None,
            8 => reports
                .results
                .get_mut(&id)
                .unwrap()
                .1
                .slots
                .as_mut()
                .unwrap()
                .clear(),
            9 => checker.points[point].owner = owner + 1,
            10 => {
                checker
                    .endpoints
                    .get_mut(&SequenceSource::Expr(point))
                    .unwrap()[1]
                    .to = Port::Normal(*other);
            }
            11 => {
                reports.blocks.remove(&id);
            }
            _ => unreachable!(),
        }
        let effects = reports.effects.clone();
        let blocks = reports.blocks.clone();
        let results = reports.results.clone();
        let index = reports.consumers.clone();
        let parts = reports.parts;
        assert!(
            checker
                .result_consumers(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(reports.effects, effects);
        assert_eq!(reports.blocks, blocks);
        assert_eq!(reports.results, results);
        assert_eq!(reports.consumers, index);
        assert_eq!(reports.parts, parts);
    }
}

#[test]
pub(crate) fn result_consumers_share_exact_map_and_work_limits_without_payload() {
    let (mut checker, mut reports) = checked("a:{->1};b:{->2}");
    let expected = reports.consumers.clone();
    let base = reports.effects.len() + reports.blocks.len() + reports.results.len();
    let limit = base + expected.len();
    reports.parts = 0;
    let before = checker.flow.work;
    assert_eq!(
        checker
            .result_consumers_limited(&reports, Span::default(), limit)
            .unwrap(),
        expected
    );
    let work = checker.flow.work - before;
    for room in [limit - 1, base - 1] {
        assert!(
            checker
                .result_consumers_limited(&reports, Span::default(), room)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let actual = checker.result_consumers_limited(&reports, Span::default(), limit);
        assert_eq!(actual.is_ok(), short == 0);
        if let Ok(index) = actual {
            assert_eq!(index, expected);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        }
    }
    assert_eq!(reports.consumers, expected);
    assert_eq!(reports.parts, 0);
}

#[test]
pub(crate) fn result_consumers_distinguish_program_root_and_function_span_bounds() {
    let (mut checker, mut reports) = checked("value:{->1};f<int32>:(){->2}");
    let root = reports.entries[&0].0;
    checker.bodies.get_mut(&root).unwrap().span = Span::default();
    reports.blocks.get_mut(&root).unwrap().1.span = Span::default();
    assert_eq!(
        checker.result_consumers(&reports, Span::default()).unwrap(),
        reports.consumers
    );
    let root = reports.entries[&1].0;
    checker.bodies.get_mut(&root).unwrap().span = Span::default();
    reports.blocks.get_mut(&root).unwrap().1.span = Span::default();
    assert!(checker.result_consumers(&reports, Span::default()).is_err());
}
