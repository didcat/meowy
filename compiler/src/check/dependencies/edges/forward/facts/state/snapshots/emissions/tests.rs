use super::super::capture::tests::capture;
use super::*;

#[test]
pub(crate) fn scalar_emission_values_preserve_tail_writes_without_publishing_the_block() {
    let source = "x:=7;{copy:x;->copy;x=9;x};x";
    let (mut checker, _, _, snapshots) = capture(source);
    let (_, op) = checker.emissions.first_key_value().unwrap();
    let id = op.targets[0].id;
    let target = op.targets[0].block;
    let start = source.find("x};").unwrap();
    let tail = *checker
        .local_reads
        .iter()
        .find(|(point, read)| {
            checker.points[**point].span == Span::new(start, start + 1) && read.owner == 0
        })
        .unwrap()
        .0;
    let value = snapshots.expressions[&tail].value.as_ref().unwrap();
    assert_eq!(
        value.domain,
        Domain::integer(9, &mut checker.flow, Span::default()).unwrap()
    );
    let emitted = snapshots.emissions[&id].value.as_ref().unwrap();
    assert_eq!(
        emitted,
        snapshots.expressions[&op.input].value.as_ref().unwrap()
    );
    assert_eq!(
        emitted.domain,
        Domain::integer(7, &mut checker.flow, Span::default()).unwrap()
    );
    assert_ne!(emitted.version, value.version);
    assert_eq!(
        snapshots.rows[&Port::Emission(id)].before,
        snapshots.rows[&Port::Emission(id)].after
    );
    assert_eq!(
        snapshots.after(Port::BlockResult(target)),
        Some(View::Missing)
    );
    let parent = checker.bodies[&target].parent.unwrap();
    assert!(!snapshots.expressions.contains_key(&parent));
}

#[test]
pub(crate) fn scalar_emission_values_keep_stopped_tails_and_later_emissions_unavailable() {
    let (mut checker, _, _, snapshots) = capture("d:@\"debug\";v:{->7;d.panic(\"stop\")};{->9}");
    assert_eq!(snapshots.emissions.len(), 1);
    let (&id, saved) = snapshots.emissions.iter().next().unwrap();
    assert_eq!(
        saved.value.as_ref().unwrap().domain,
        Domain::integer(7, &mut checker.flow, Span::default()).unwrap()
    );
    let (statement, _) = checker.emission_sources[&id];
    let block = checker.emissions[&statement].targets[0].block;
    assert_eq!(snapshots.before(Port::BlockResult(block)), None);
    let (_, _, _, snapshots) = capture("'out{->{'out.leave()}}");
    assert!(snapshots.emissions.is_empty());
}

#[test]
pub(crate) fn scalar_emission_values_keep_unsupported_sources_and_compositions_conservative() {
    for source in ["x:1;{->&x}", "{->1.5}", "{->\"cat\"}"] {
        let (_, _, _, snapshots) = capture(source);
        assert_eq!(snapshots.emissions.len(), 1, "{source}");
        let (&id, fact) = snapshots.emissions.iter().next().unwrap();
        assert!(fact.value.is_none(), "{source}");
        assert_eq!(snapshots.after(Port::Emission(id)), Some(View::Missing));
    }
    let (_, _, _, snapshots) = capture("{->{->a:1}}");
    assert!(snapshots.emissions.is_empty());
}
