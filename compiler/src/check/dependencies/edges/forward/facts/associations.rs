use super::*;
use crate::check::dependencies::{
    OperationKind,
    edges::forward::{effects::Effect, entries::tests::checked},
};

#[test]
pub(crate) fn scalar_copy_sources_keep_bind_read_and_parameter_associations_distinct() {
    let source = "a:7;b:a;c:b;f<int32>:(n<int32>,other<int32>){copy:n;->copy}";
    crate::compile(source).unwrap();
    let (mut checker, program) = checked(source);
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let (literals, declared, _) = checker
        .scalar_seed_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let reads = checker.local_reads.clone();
    for (point, read) in reads {
        let parent = checker.points[point].parent.unwrap();
        assert_eq!(
            checker
                .unchanged_narrowing_input(&reports, parent, read.owner, Span::default())
                .unwrap(),
            Some(point)
        );
        assert!(!literals.contains_key(&point));
        let input = checker
            .read_initializer_input(&reports, point, read.owner, Span::default())
            .unwrap();
        if let Some((function, _, _)) = declared.params.get(&read.local) {
            assert_eq!(*function + 1, read.owner);
            assert!(input.is_none());
            assert!(!reports.initializers.contains_key(&read.local));
        } else {
            let init = reports.initializers[&read.local];
            let binding = checker
                .binding_effect(&reports, init.statement, init.owner, Span::default())
                .unwrap()
                .unwrap();
            assert_eq!(binding.local, read.local);
            assert_eq!(binding.storage, read.storage);
            assert_eq!(input, binding.input);
            assert_ne!(point, init.statement);
        }
    }
    let params = &program.functions[0].params;
    assert_eq!(
        declared.params[&params[0]].2.domain,
        declared.params[&params[1]].2.domain
    );
    assert_ne!(params[0], params[1]);
}

#[test]
pub(crate) fn scalar_copy_sources_do_not_recover_mutable_history_from_initializers() {
    let source = "x:=1;before:x;x=2;after:x;stable:3;copy:stable";
    crate::compile(source).unwrap();
    let (mut checker, program) = checked(source);
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let local = checker
        .operations
        .values()
        .find(|op| op.kind == OperationKind::Write)
        .unwrap()
        .local;
    assert!(!reports.eligible.contains(&local));
    assert!(!reports.initializers.contains_key(&local));
    let points = checker
        .local_reads
        .iter()
        .filter_map(|(&point, read)| (read.local == local).then_some(point))
        .collect::<Vec<_>>();
    assert_eq!(points.len(), 2);
    for point in points {
        assert!(
            checker
                .read_initializer_input(&reports, point, 0, Span::default())
                .unwrap()
                .is_none()
        );
    }
    let copies = reports
        .effects
        .values()
        .filter(|(_, effect)| matches!(effect, Effect::Read { .. }))
        .count();
    assert_eq!(copies, 3);
}

#[test]
pub(crate) fn scalar_copy_sources_require_observed_bind_and_read_links() {
    let source = "a:7;copy:a;d:@\"debug\";stop:d.panic(\"stop\");late:9";
    crate::compile(source).unwrap();
    let (mut checker, program) = checked(source);
    let mut reports = checker.entry_reports(&program, Span::default()).unwrap();
    let point = *checker.local_reads.first_key_value().unwrap().0;
    assert!(
        checker
            .read_initializer_input(&reports, point, 0, Span::default())
            .unwrap()
            .is_some()
    );
    reports.effects.remove(&point);
    assert!(
        checker
            .read_initializer_input(&reports, point, 0, Span::default())
            .unwrap()
            .is_none()
    );
    let binds = checker.operations.keys().copied().collect::<Vec<_>>();
    assert_eq!(binds.len(), 4);
    for point in &binds[2..] {
        assert!(
            checker
                .binding_effect(&reports, *point, 0, Span::default())
                .unwrap()
                .is_none()
        );
    }
    let (seeds, _, _) = checker
        .scalar_seed_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(
        seeds.values().filter(|seed| seed.value.is_some()).count(),
        1
    );
}
