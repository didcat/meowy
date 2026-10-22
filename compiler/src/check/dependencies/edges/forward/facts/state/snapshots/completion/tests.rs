use super::super::capture::tests::capture;
use super::*;

#[test]
pub(crate) fn scalar_completion_storage_preserves_empty_scopes_tail_writes_and_direct_leaves() {
    for source in ["x:=1;{};{x=2};x", "x:=1;'out{x=2;'out.leave()};x"] {
        let (mut checker, _, _, snapshots) = capture(source);
        let point = *checker.local_reads.keys().next_back().unwrap();
        assert_eq!(
            snapshots.expressions[&point].value.as_ref().unwrap().domain,
            Domain::integer(2, &mut checker.flow, Span::default()).unwrap()
        );
        for body in checker.bodies.values() {
            if let Some(parent) = body.parent {
                assert!(!snapshots.expressions.contains_key(&parent));
            }
        }
    }
}

#[test]
pub(crate) fn scalar_completion_storage_joins_guarded_leave_and_fallthrough_versions() {
    let (mut checker, _, _, snapshots) =
        capture("f:(flag<boolean>){x:=1;'out{|flag|'out.leave();x=2};x}");
    let point = *checker.local_reads.keys().next_back().unwrap();
    let expected = Domain::integer(1, &mut checker.flow, Span::default())
        .unwrap()
        .join(
            &Domain::integer(2, &mut checker.flow, Span::default()).unwrap(),
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    let value = snapshots.expressions[&point].value.as_ref().unwrap();
    assert_eq!(value.domain, expected);
    assert_eq!(value.version, None);
    assert_eq!(value.origin, None);
    assert!(!value.control);
}

#[test]
pub(crate) fn scalar_completion_storage_uses_qualified_boolean_paths_and_preserves_copies() {
    for (flag, expected) in [("true", 1), ("false", 2)] {
        let source = format!("x:=1;copy:x;'out{{|{flag}|'out.leave();x=2}};copy;x");
        let (mut checker, _, _, snapshots) = capture(&source);
        let point = *checker.local_reads.keys().next_back().unwrap();
        assert_eq!(
            snapshots.expressions[&point].value.as_ref().unwrap().domain,
            Domain::integer(expected, &mut checker.flow, Span::default()).unwrap()
        );
        for (&point, read) in &checker.local_reads {
            if read.local == 1 {
                assert_eq!(
                    snapshots.expressions[&point].value.as_ref().unwrap().domain,
                    Domain::integer(1, &mut checker.flow, Span::default()).unwrap()
                );
            }
        }
    }
}

#[test]
pub(crate) fn scalar_completion_storage_keeps_unknown_effects_dead_tails_and_restarts_unavailable()
{
    for source in [
        "d:@\"debug\";x:=1;{d.print(\"tail\");x=2};x",
        "x:=1;'out{'out.leave();1};x",
        "x:=1;'loop{|false|'loop.restart();x=2};x",
        "x:=1;{->field:2;x=2};x",
        "x:=1;'out{|true|{'out.leave()};x=2};x",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let point = *checker.local_reads.keys().next_back().unwrap();
        assert!(snapshots.expressions[&point].value.is_none(), "{source}");
    }
}
