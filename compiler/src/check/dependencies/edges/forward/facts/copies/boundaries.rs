use super::super::{domain::Domain, versions::tests::context};
use super::*;
use crate::check::dependencies::edges::forward::effects::Effect;

#[test]
pub(crate) fn scalar_copy_boundaries_preserve_control_marks_without_changing_dependency_authority()
{
    let source = "seed:false;|seed|{a:7;b:a;c:b}";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    checker.derived.insert(0);
    let body = checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    let program = hir::Program {
        body,
        functions: Vec::new(),
        locals: std::mem::take(&mut checker.locals),
    };
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let marks = checker.derived.clone();
    let (versions, _) = checker
        .scalar_copy_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let marked = versions
        .values()
        .filter(|slot| {
            matches!(
                slot.source,
                Some((_, Effect::Storage { control: true, .. }))
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(marked.len(), 3);
    assert!(
        marked
            .iter()
            .all(|slot| slot.value.as_ref().unwrap().control)
    );
    assert!(marked.iter().all(|slot| slot.value == marked[0].value));
    assert_eq!(checker.derived, marks);
}

#[test]
pub(crate) fn scalar_copy_boundaries_ignore_mutable_frontend_caches_and_later_unrelated_writes() {
    let (mut checker, program, reports) =
        context("a:7;copy:a;x:=1;x=2;after:copy;f<int32>:(n<int32>){local:n;->local}");
    let (versions, _) = checker
        .scalar_copy_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let before = versions
        .into_iter()
        .map(|(local, slot)| (local, (slot.version, slot.value)))
        .collect::<BTreeMap<_, _>>();
    let marks = checker.derived.clone();
    checker.constants.clear();
    checker.inputs.clear();
    checker.bool_inputs.clear();
    checker.record_inputs.clear();
    checker.bools.clear();
    checker.tags.clear();
    checker.pointees.clear();
    let (versions, _) = checker
        .scalar_copy_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let after = versions
        .into_iter()
        .map(|(local, slot)| (local, (slot.version, slot.value)))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(before, after);
    assert_eq!(after[&0].1, after[&1].1);
    assert_eq!(after[&1].1, after[&3].1);
    assert!(!after.contains_key(&2));
    assert_eq!(checker.derived, marks);
}

#[test]
pub(crate) fn scalar_copy_boundaries_keep_missing_inputs_and_observations_unavailable() {
    for fault in 0..4 {
        let (mut checker, program, mut reports) = context("a:7;b:a");
        let literal = *checker.scalar_leaves.first_key_value().unwrap().0;
        match fault {
            0 => {
                reports.effects.remove(&literal);
            }
            1 => {
                let (_, Effect::Scalar(value)) = reports.effects.get_mut(&literal).unwrap() else {
                    panic!()
                };
                value.result = false;
            }
            2 => {
                reports
                    .effects
                    .remove(checker.local_reads.first_key_value().unwrap().0);
            }
            3 => {
                let init = reports.initializers.get_mut(&0).unwrap();
                init.input = None;
                let op = checker.operations.get_mut(&init.statement).unwrap();
                op.input = None;
                op.edges = vec![Edge::new(
                    Port::Operation(init.statement),
                    Port::Normal(init.statement),
                    Route::Next,
                )];
                let (_, Effect::Storage { input, .. }) =
                    reports.effects.get_mut(&init.statement).unwrap()
                else {
                    panic!()
                };
                *input = None;
            }
            _ => unreachable!(),
        }
        let (versions, _) = checker
            .scalar_copy_context(&program, &reports, Span::default(), MAX_EDGES)
            .unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[&0].value.is_some(), fault == 2);
        assert!(versions[&1].value.is_none());
        assert!(
            versions
                .values()
                .all(|slot| slot.declared.is_some() && slot.declared != Some(Domain::BOTTOM))
        );
    }
}

#[test]
pub(crate) fn scalar_copy_boundaries_keep_emitted_aliases_distinct_from_ordinary_versions() {
    let (mut checker, program, reports) =
        context("flag:false;row:'out{|flag|{'out->n:1;copy:n};|!flag|{'out->n:2;copy:n}}");
    let (versions, _) = checker
        .scalar_copy_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(
        versions
            .values()
            .filter(|slot| slot.value.is_some())
            .count(),
        1
    );
    let aliases = checker
        .local_reads
        .iter()
        .filter_map(|(&point, read)| {
            checker
                .proofs
                .aliases
                .contains_key(&read.local)
                .then_some((point, read.owner))
        })
        .collect::<Vec<_>>();
    assert_eq!(aliases.len(), 2);
    for (point, owner) in aliases {
        assert!(
            checker
                .scalar_read_version(&reports, &versions, point, owner, Span::default())
                .unwrap()
                .is_none()
        );
    }
}
