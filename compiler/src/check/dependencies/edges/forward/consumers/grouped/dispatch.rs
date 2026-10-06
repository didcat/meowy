use super::*;
use crate::check::dependencies::edges::forward::consumers::tests::checked;

#[test]
pub(crate) fn dispatch_consumers_follow_checked_wrappers_without_reclassifying_blocks() {
    let (mut checker, reports) = checked(
        "n<int32>:((3.{->$})~<int32>);copy:n;s:{->((copy))};f<int32>:(){n:4.{->$};->((n))}",
    );
    let inputs: Vec<_> = checker
        .operations
        .values()
        .filter_map(|op| op.input.map(|point| (op.owner, point)))
        .chain(
            reports
                .candidate_inputs
                .values()
                .map(|(owner, input)| (*owner, input.point)),
        )
        .collect();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let mut owners = BTreeSet::new();
    let mut wrapped = 0;
    for (owner, input) in inputs {
        let Some(id) = checker
            .dispatch_consumer(&reports, input, owner, Span::default())
            .unwrap()
        else {
            continue;
        };
        assert_eq!(checker.dispatch_ops[&id].owner, owner);
        assert!(!reports.consumers.contains_key(&id));
        assert_eq!(
            checker
                .grouped_consumer(&reports, input, owner, Span::default())
                .unwrap(),
            None
        );
        owners.insert(owner);
        wrapped += usize::from(input != id);
    }
    assert_eq!(owners, BTreeSet::from([0, 1]));
    assert!(wrapped >= 2);
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn dispatch_consumers_keep_blocks_calls_fields_and_changed_wrappers_opaque() {
    for source in [
        "n:{->1};s:{->n}",
        "r:{->n:1};s:{->r.n}",
        "f<int32>:(){->1};s:{->f()}",
        "<U>:<int32><null>;n:3.{->$};s:{->(n~<U>)}",
        "n:=3.{->$};s:{->n}",
    ] {
        let (mut checker, reports) = checked(source);
        let inputs: Vec<_> = reports
            .candidate_inputs
            .values()
            .map(|(owner, input)| (*owner, input.point))
            .collect();
        for (owner, input) in inputs {
            assert_eq!(
                checker
                    .dispatch_consumer(&reports, input, owner, Span::default())
                    .unwrap(),
                None,
                "{source}"
            );
        }
    }
}

#[test]
pub(crate) fn dispatch_consumers_share_exact_wrapper_hops_and_work() {
    let depth = 16;
    let (mut checker, reports) = checked(&format!(
        "v:3.{{->$}};s:{{->{}v{}}}",
        "(".repeat(depth),
        ")".repeat(depth)
    ));
    let direct = reports
        .direct_sources
        .values()
        .find(|(_, direct)| direct.dispatch.is_some())
        .unwrap()
        .1;
    let id = direct.dispatch.unwrap().point;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for (input, hops, allowed) in [
        (id, 0, true),
        (direct.point, depth + 2, true),
        (direct.point, depth + 1, false),
    ] {
        let result = checker.grouped_source_limited(
            &reports,
            input,
            0,
            Span::default(),
            hops,
            Target::Dispatch,
        );
        if allowed {
            assert_eq!(result.unwrap(), Some(id));
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
    let work = checker.flow.work;
    assert_eq!(
        checker
            .dispatch_consumer(&reports, direct.point, 0, Span::default())
            .unwrap(),
        Some(id)
    );
    let work = checker.flow.work - work;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.dispatch_consumer(&reports, direct.point, 0, Span::default());
        if short == 0 {
            assert_eq!(result.unwrap(), Some(id));
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
