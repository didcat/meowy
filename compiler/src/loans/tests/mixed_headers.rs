use super::access::inspect;
use crate::flow::{FALSE, TRUE};
use crate::hir::ReferenceMode;

pub(crate) const SOURCE: &str = "<R>:<{n<int32>:=}>;x:1;y:2;p:=&x;first:=true;i:=0;r<R>:'out{'loop{|first|{'out->n:=7;w:&!n;*w=8;first=false};v:*p;p=&y;i=i+1;|i<2|'loop.restart()}}";

#[test]
pub(crate) fn mixed_headers_certify_predecessors_without_granting_precise_authority() {
    inspect(SOURCE, |graph, reach| {
        let headers = graph.shared_restart_headers(reach).unwrap();
        assert!(headers.len() >= 2);
        graph.exclusive_restart_frontiers(reach).unwrap();
        graph.solve_authority(reach).unwrap();
        let mut seen = false;
        for id in headers {
            for value in &graph.nodes[id].opaque {
                seen = true;
                assert_eq!(graph.authority[*value].opaque, TRUE);
            }
        }
        assert!(seen);
        let loan = graph
            .loans
            .iter()
            .find(|loan| loan.mode == ReferenceMode::Exclusive)
            .unwrap();
        assert_eq!(graph.authority[loan.value].opaque, FALSE);
    });
}

#[test]
pub(crate) fn mixed_headers_reject_missing_entry_and_backedge_transfers() {
    for backedge in [false, true] {
        inspect(SOURCE, |graph, reach| {
            let node = graph
                .nodes
                .iter_mut()
                .find(|node| {
                    node.header
                        .as_ref()
                        .is_some_and(|header| !header.required.is_empty())
                        && node.opaque.is_empty() == backedge
                })
                .unwrap();
            node.transfers.clear();
            let error = graph.shared_restart_headers(reach).unwrap_err();
            assert_eq!(error.code, "B001");
            assert!(error.message.contains("header coverage"));
        });
    }
}

#[test]
pub(crate) fn mixed_headers_cannot_hide_missing_definitions_or_explicit_opacity() {
    for mode in 0..3 {
        inspect(SOURCE, |graph, reach| {
            let id = graph
                .nodes
                .iter()
                .position(|node| {
                    node.header
                        .as_ref()
                        .is_some_and(|header| !header.required.is_empty())
                        && node.opaque.is_empty()
                })
                .unwrap();
            if mode == 0 {
                graph.nodes[id].header = None;
            } else if mode == 1 {
                graph.nodes[id].header.as_mut().unwrap().required.clear();
                graph.nodes[id].defs.clear();
                graph.nodes[id].transfers.clear();
            } else {
                let value = *graph.nodes[id]
                    .header
                    .as_ref()
                    .unwrap()
                    .required
                    .keys()
                    .next()
                    .unwrap();
                graph.nodes[0].opaque.push(value);
                graph.shared_restart_headers(reach).unwrap();
            }
            assert_eq!(
                graph.exclusive_restart_frontiers(reach).unwrap_err().code,
                "B001"
            );
        });
    }
}

#[test]
pub(crate) fn mixed_headers_allow_inactive_paths_and_reject_invalid_or_exhausted_proofs() {
    inspect(
        "<R>:<{n<int32>:=}>;<H>:<{view<&int32><null>}>;x:7;empty<H>:{};full<H>:{->view:&x};h:=empty;first:=true;i:=0;r<R>:'out{'loop{|first|{'out->n:=7;w:&!n;*w=8;first=false};|h.view<&int32>|v:*(h.view~<&int32>);h=full;i=i+1;|i<2|'loop.restart()}}",
        |graph, reach| {
            assert!(
                graph
                    .nodes
                    .iter()
                    .filter_map(|node| node.header.as_ref())
                    .any(|header| header.required.values().any(|active| *active == FALSE))
            );
            graph.shared_restart_headers(reach).unwrap();
            graph.exclusive_restart_frontiers(reach).unwrap();
        },
    );
    for budget in [false, true] {
        inspect(SOURCE, |graph, reach| {
            if budget {
                graph.guards.spend(usize::MAX);
            } else {
                let count = graph.values.len();
                graph
                    .nodes
                    .iter_mut()
                    .find_map(|node| node.header.as_mut())
                    .unwrap()
                    .required
                    .insert(count, TRUE);
            }
            assert_eq!(
                graph.shared_restart_headers(reach).unwrap_err().code,
                "B001"
            );
        });
    }
}
