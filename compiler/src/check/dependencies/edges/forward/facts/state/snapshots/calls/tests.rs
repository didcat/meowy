use super::super::capture::tests::capture;
use super::*;
use domain::Values;

#[test]
pub(crate) fn scalar_call_capture_preserves_unaffected_values_and_old_copies() {
    for source in [
        "f:(){};x:=false;old:x;f();x;old",
        "f:(n<boolean>){};x:=false;old:x;f(true);x;old",
        "f:(p<&boolean>){};x:=false;old:x;f(&x);x;old",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let reads = checker
            .local_reads
            .iter()
            .filter(|(_, read)| read.owner == 0)
            .collect::<Vec<_>>();
        let before = snapshots.expressions[reads[0].0].value.as_ref().unwrap();
        let after = snapshots.expressions[reads[reads.len() - 2].0]
            .value
            .as_ref()
            .unwrap();
        let copy = snapshots.expressions[reads[reads.len() - 1].0]
            .value
            .as_ref()
            .unwrap();
        assert_eq!(after.domain, before.domain);
        assert_eq!(after.version, before.version);
        assert_eq!(after.origin, before.origin);
        assert_eq!(copy.domain, before.domain);
        assert_eq!(copy.origin, before.origin);
    }
}

#[test]
pub(crate) fn scalar_call_capture_resets_direct_targets_without_inspecting_empty_bodies() {
    let (checker, _, _, snapshots) =
        capture("f:(p<&!boolean>){};x:=false;y:=true;old:x;f(((&!x)));x;y;old");
    let reads = checker
        .local_reads
        .iter()
        .filter(|(_, read)| read.owner == 0)
        .collect::<Vec<_>>();
    let before = snapshots.expressions[reads[0].0].value.as_ref().unwrap();
    let after = snapshots.expressions[reads[1].0].value.as_ref().unwrap();
    assert_eq!(before.domain, Domain::boolean(false));
    assert_eq!(
        after.domain,
        Domain {
            values: Some(Values::Bool(3)),
            null: false
        }
    );
    assert_eq!((after.version, after.origin), (None, None));
    assert_eq!(after.local, before.local);
    assert!(!after.control);
    assert_eq!(
        snapshots.expressions[reads[2].0]
            .value
            .as_ref()
            .unwrap()
            .domain,
        Domain::boolean(true)
    );
    let copy = snapshots.expressions[reads[3].0].value.as_ref().unwrap();
    assert_eq!(copy.domain, before.domain);
    assert_eq!(copy.origin, before.origin);
    let call = checker
        .invocations
        .values()
        .find(|call| call.owner == 0)
        .unwrap();
    let Some(View::Known(earlier)) = snapshots.before(Port::Operation(call.point)) else {
        panic!()
    };
    assert!(
        matches!(&earlier.get(before.local.unwrap()).unwrap().data, Data::Initialized { version: Some(_), domain: Some(domain), .. } if *domain == Domain::boolean(false))
    );
}

#[test]
pub(crate) fn scalar_call_capture_never_restores_missing_state_or_unknown_targets() {
    for source in [
        "f:(){};x:=false;5+6;f();x",
        "f:(p<&!boolean>){};x:=false;p:&!x;f(p);x",
        "f:(){};x:=false;f();r:={->n:=1};r.n=2;x",
        "f:(){};x:=false;f();*(&!x)=true;x",
        "f:(){};x:=false;f();row:{->n:=1;n=2};x",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let read = *checker.local_reads.keys().next_back().unwrap();
        assert!(snapshots.expressions[&read].value.is_none());
    }
}

#[test]
pub(crate) fn scalar_call_capture_joins_conditional_mutations_without_changing_copies() {
    let (checker, _, _, snapshots) =
        capture("f:(p<&!boolean>){};g:(flag<boolean>){x:=false;old:x;|flag|f(&!x);x;old}");
    let reads = checker.local_reads.iter().rev().take(2).collect::<Vec<_>>();
    let copy = snapshots.expressions[reads[0].0].value.as_ref().unwrap();
    let current = snapshots.expressions[reads[1].0].value.as_ref().unwrap();
    assert_eq!(copy.domain, Domain::boolean(false));
    assert_eq!(current.domain.values, Some(Values::Bool(3)));
    assert_eq!((current.version, current.origin), (None, None));
    assert!(copy.version.is_some() && copy.origin.is_some());
}

#[test]
pub(crate) fn scalar_call_capture_never_invalidates_after_a_stopped_argument() {
    let (checker, _, _, snapshots) =
        capture("d:@\"debug\";f:(p<&!boolean>,b<boolean>){};x:=false;f(&!x,d.panic(\"stop\"))");
    let call = checker
        .invocations
        .values()
        .find(|call| call.args.len() == 2)
        .unwrap();
    assert!(matches!(
        snapshots.after(Port::Normal(call.args[0])),
        Some(View::Known(_))
    ));
    assert_eq!(snapshots.after(Port::Operation(call.point)), None);
    assert_eq!(snapshots.after(Port::Normal(call.point)), None);
}
