use super::*;
use crate::check::dependencies::edges::forward::effects::tests::checked;

pub(super) const SOURCE: &str = "flag:=false;r:3.{|flag|->$;|!flag|->4;->tag:true}";

#[test]
pub(crate) fn receiver_inputs_preserve_guarded_emissions_nested_blocks_and_owners() {
    for source in [
        SOURCE,
        "f<int32>:(flag<boolean>){->3.{|flag|->$;|!flag|->4}}",
        "flag:=false;other:=true;r:3.{|flag|{|other|copy:$;->$};|!flag|->4;->tag:true}",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, reports) = checked(source, false);
        let reads: Vec<_> = checker
            .local_reads
            .iter()
            .filter_map(|(&id, read)| {
                reports
                    .receivers
                    .get(&read.local)
                    .map(|&(owner, dispatch, _)| (id, owner, dispatch))
            })
            .collect();
        assert!(!reads.is_empty());
        for (id, owner, dispatch) in reads {
            let input = checker.dispatch_ops[&dispatch].input;
            assert_eq!(
                checker
                    .read_receiver_input(&reports, id, owner, Span::default())
                    .unwrap(),
                Some(input)
            );
        }
    }
}

#[test]
pub(crate) fn receiver_guard_scopes_reject_wrong_branch_edges_sites_and_condition_identities() {
    for fault in 0..10 {
        let (mut checker, reports) = checked(SOURCE, false);
        let (&read, _) = checker
            .local_reads
            .iter()
            .find(|(_, read)| reports.receivers.contains_key(&read.local))
            .unwrap();
        let mut arm = read;
        while checker.points[arm].kind != PointKind::Then {
            arm = checker.points[arm].parent.unwrap();
        }
        let branch = checker.points[arm].parent.unwrap();
        let site = checker.points[arm].site.unwrap();
        let Port::Entry(condition) = checker.branch_edges[&branch][0].to else {
            panic!()
        };
        match fault {
            0 => {
                checker.branch_edges.remove(&branch);
            }
            1 => checker.branch_edges.get_mut(&branch).unwrap()[1].to = Port::Entry(branch),
            2 => checker.branch_edges.get_mut(&branch).unwrap()[1].route = Route::False,
            3 => checker.points[arm].site = None,
            4 => checker.points[branch].site = None,
            5 => checker.sites.get_mut(&site).unwrap().span.end = checker.points[arm].span.start,
            6 => checker.points[condition].kind = PointKind::Expr,
            7 => checker.points[condition].parent = None,
            8 => checker.points[condition].owner += 1,
            9 => checker.sites.get_mut(&site).unwrap().owner += 1,
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .read_receiver_input(&reports, read, 0, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn receiver_guard_scopes_preserve_exact_hop_and_work_bounds() {
    let (mut checker, reports) = checked(SOURCE, false);
    let (&read, op) = checker
        .local_reads
        .iter()
        .find(|(_, read)| reports.receivers.contains_key(&read.local))
        .unwrap();
    let block = checker.dispatch_ops[&reports.receivers[&op.local].1].block;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    checker
        .receiver_scope(read, block, 0, Span::default(), 6)
        .unwrap();
    let work = checker.flow.work - start;
    assert!(
        checker
            .receiver_scope(read, block, 0, Span::default(), 5)
            .unwrap_err()
            .message
            .contains("budget")
    );
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.receiver_scope(read, block, 0, Span::default(), 6);
        assert_eq!(result.is_ok(), short == 0);
        if short == 1 {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
