use super::access::inspect_body;
use crate::borrow::Source;
use crate::flow::{FALSE, Guard, TRUE};
use crate::loans::access::{Kind, Target};
use crate::loans::authority::{Authority, LoanId};
use crate::loans::{Graph, Node};

pub(crate) fn inspect(source: &str, check: impl FnOnce(&mut Graph<'_>, &[Guard])) {
    inspect_body(source, None, |graph, reach| {
        graph.solve_authority(reach).unwrap();
        check(graph, reach);
    });
}

pub(crate) fn reads(graph: &Graph<'_>, reach: &[Guard]) -> Vec<usize> {
    graph
        .nodes
        .iter()
        .zip(reach)
        .filter_map(|(node, guard)| {
            let access = node.access.as_ref()?;
            match access.target {
                Target::Pointee(value) if access.kind == Kind::Read && *guard != FALSE => {
                    Some(value)
                }
                _ => None,
            }
        })
        .collect()
}

#[test]
pub(crate) fn copies_keep_one_acquisition_identity_across_distinct_value_versions() {
    inspect("a:1;p:&a;q:p;r:q;v:*p;w:*r", |graph, reach| {
        assert_eq!(graph.loans.len(), 1);
        let values = reads(graph, reach);
        assert_eq!(values.len(), 2);
        assert_ne!(values[0], values[1]);
        for value in values {
            assert_eq!(
                graph.authority[value]
                    .loans
                    .keys()
                    .copied()
                    .collect::<Vec<_>>(),
                [LoanId(0)]
            );
            assert_eq!(graph.authority[value].opaque, FALSE);
        }
    });
}

#[test]
pub(crate) fn separate_acquisitions_to_one_address_never_merge_identity() {
    inspect("a:1;p:&a;q:&a;v:*p;w:*q", |graph, reach| {
        assert_eq!(graph.loans.len(), 2);
        let values = reads(graph, reach);
        assert_eq!(values.len(), 2);
        assert_ne!(
            graph.authority[values[0]].loans,
            graph.authority[values[1]].loans
        );
        assert_eq!(
            graph.values[values[0]].origins[0].source,
            graph.values[values[1]].origins[0].source
        );
    });
}

#[test]
pub(crate) fn reborrow_chains_and_child_copies_preserve_parent_authority() {
    inspect("a:1;p:&a;q:&*p;r:&*q;s:r;v:*s", |graph, reach| {
        assert_eq!(graph.loans.len(), 3);
        assert!(graph.loans[0].parent.is_none());
        for id in 1..3 {
            assert!(graph.loans[id].parent.is_some());
            assert_eq!(
                graph.loans[id]
                    .parents
                    .loans
                    .keys()
                    .copied()
                    .collect::<Vec<_>>(),
                [LoanId(id - 1)]
            );
            assert_eq!(graph.loans[id].parents.opaque, FALSE);
        }
        assert_eq!(
            graph.authority[reads(graph, reach)[0]]
                .loans
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            [LoanId(2)]
        );
    });
}

#[test]
pub(crate) fn replacing_a_handle_does_not_retarget_an_existing_child() {
    inspect("a:1;b:2;p:=&a;s:&*p;p=&b;t:s;v:*t;w:*p", |graph, reach| {
        let child = graph
            .loans
            .iter()
            .position(|loan| loan.parent.is_some())
            .unwrap();
        let values = reads(graph, reach);
        assert_eq!(values.len(), 2);
        assert!(
            graph.authority[values[0]]
                .loans
                .contains_key(&LoanId(child))
        );
        assert!(
            !graph.authority[values[1]]
                .loans
                .contains_key(&LoanId(child))
        );
        assert_eq!(
            graph.loans[child]
                .parents
                .loans
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            [LoanId(0)]
        );
    });
}

#[test]
pub(crate) fn guarded_replacement_keeps_disjoint_parent_alternatives() {
    inspect(
        "a:1;b:2;p:=&a;flag:=true;|flag|p=&b;s:&*p;v:*s",
        |graph, _| {
            let child = graph
                .loans
                .iter()
                .find(|loan| loan.parent.is_some())
                .unwrap();
            let guards = child.parents.loans.values().copied().collect::<Vec<_>>();
            assert_eq!(guards.len(), 2);
            assert_eq!(child.parents.opaque, FALSE);
            assert!(!graph.guards.overlap(guards[0], guards[1]));
            assert_eq!(graph.guards.or(guards[0], guards[1]), TRUE);
        },
    );
}

#[test]
pub(crate) fn short_circuits_and_leaves_preserve_only_actual_parent_predecessors() {
    for source in [
        "a:1;b:2;p:=&a;flag:=true;v:flag&&{p=&b;->true};s:&*p;w:*s",
        "a:1;b:2;p:=&a;flag:=true;'out{|flag|{p=&b;'out.leave()};p=&a};s:&*p;w:*s",
    ] {
        inspect(source, |graph, _| {
            let child = graph
                .loans
                .iter()
                .find(|loan| loan.parent.is_some())
                .unwrap();
            let guards = child.parents.loans.values().copied().collect::<Vec<_>>();
            assert_eq!(guards.len(), 2);
            assert_eq!(child.parents.opaque, FALSE);
            assert!(!graph.guards.overlap(guards[0], guards[1]));
            assert_eq!(graph.guards.or(guards[0], guards[1]), TRUE);
        });
    }
}

#[test]
pub(crate) fn field_accesses_resolve_to_the_same_region_through_distinct_views() {
    inspect(
        "a:={->n:=1};p:&a;q:&(p.n);v:a.n;w:p.n;z:*q",
        |graph, reach| {
            let regions = graph.nodes.iter().zip(reach).filter_map(|(node, guard)| {
            let access = node.access.as_ref()?;
            if *guard == FALSE || access.kind != Kind::Read { return None }
            access.regions.iter().find(|region| matches!(&region.source, Source::Local { fields, .. } if fields == &[crate::borrow_value::Projection::Field(0)])).cloned()
        }).collect::<Vec<_>>();
            assert_eq!(regions.len(), 3);
            assert!(regions.iter().all(|region| region.source == regions[0].source && region.component.is_empty()));
            let child = graph
                .loans
                .iter()
                .find(|loan| loan.parent.is_some())
                .unwrap();
            assert_eq!(child.parents.loans.len(), 1);
        },
    );
}

#[test]
pub(crate) fn public_bounds_survive_joins_without_becoming_physical_access_regions() {
    const PREFIX: &str = "first<&int32>:(p<&int32>,q<&string>){->p};a:=1;text:=\"old\";p:=first(&a,&text);flag:=true;|flag|p=first(&a,&text);";
    inspect(&format!("{PREFIX}v:*p"), |graph, reach| {
        let value = reads(graph, reach)[0];
        assert!(
            graph.values[value]
                .origins
                .iter()
                .all(|origin| matches!(origin.source, Source::Local { id: 2, .. }))
        );
        assert!(
            graph.values[value]
                .bounds
                .iter()
                .any(|origin| matches!(origin.source, Source::Local { id: 3, .. }))
        );
        let access = graph
            .nodes
            .iter()
            .filter_map(|node| node.access.as_ref())
            .find(|access| access.kind == Kind::Read && access.target == Target::Pointee(value))
            .unwrap();
        assert!(!access.regions.is_empty());
        assert!(
            access
                .regions
                .iter()
                .all(|region| matches!(region.source, Source::Local { id: 2, .. }))
        );
    });
    super::rejects(&format!("{PREFIX}text=\"new\";v:*p"), "E302");
}

#[test]
pub(crate) fn calls_and_their_reborrow_descendants_keep_opaque_ancestry() {
    inspect(
        "id<&int32>:(p<&int32>,unused<string>){->p};a:1;p:id(&a,\"\");q:&*p;r:q;v:*r",
        |graph, reach| {
            let child = graph
                .loans
                .iter()
                .find(|loan| loan.parent.is_some())
                .unwrap();
            assert_eq!(child.parents.opaque, TRUE);
            assert_eq!(graph.authority[reads(graph, reach)[0]].opaque, TRUE);
        },
    );
}

#[test]
pub(crate) fn restart_bodies_keep_static_sites_opaque_and_retain_expired_sources() {
    inspect(
        "a:1;p:=&a;i:=0;'loop{local:i;p=&local;v:*p;i=i+1;|i<2|'loop.restart()}",
        |graph, reach| {
            assert!(
                graph
                    .values
                    .iter()
                    .flat_map(|value| &value.origins)
                    .any(|origin| matches!(origin.source, Source::Expired { .. }))
            );
            for value in reads(graph, reach) {
                assert_eq!(graph.authority[value].opaque, TRUE);
            }
        },
    );
}

#[test]
pub(crate) fn restart_region_gaps_are_explicit_without_promoting_bounds_to_origins() {
    inspect(
        "<H>:<{view<&int32><null>}>;make<H>:(p<&int32>,text<&string>){->view:p};a:1;text:=\"old\";full:make(&a,&text);empty<H>:{};p:=&full;copy:*p;i:=0;'loop{p=&empty;i=i+1;|i<2|'loop.restart()};|copy.view<&int32>|v:*(copy.view~<&int32>)",
        |graph, _| {
            let access = graph
                .nodes
                .iter()
                .filter_map(|node| node.access.as_ref())
                .find(|access| access.unresolved != FALSE)
                .unwrap();
            let Target::Pointee(value) = access.target else {
                panic!("pointee")
            };
            assert_eq!(graph.authority[value].opaque, TRUE);
            let bound = graph.values[value].bounds.iter().find(|bound| {
                matches!(bound.source, Source::Local { id, .. } if graph.program.locals[id] == crate::hir::Type::String)
            }).unwrap();
            assert!(
                access
                    .regions
                    .iter()
                    .all(|region| region.source != bound.source)
            );
        },
    );
    inspect_body(
        "id<&int32>:(p<&int32>){->p};a:1;p:id(&a);v:*p",
        None,
        |graph, reach| {
            let value = reads(graph, reach)[0];
            graph.values[value].origins.clear();
            let error = graph.solve_authority(reach).unwrap_err();
            assert_eq!(error.code, "B001");
            assert!(error.message.contains("physical access origin"));
        },
    );
}

#[test]
pub(crate) fn contained_references_keep_their_loan_when_the_carrier_is_copied() {
    inspect(
        "a:1;copy:{holder:{->view:&a};outer:&holder;->*outer};v:*(copy.view)",
        |graph, reach| {
            let values = reads(graph, reach);
            assert_eq!(values.len(), 2);
            let last = *values.last().unwrap();
            assert_eq!(
                graph.authority[last]
                    .loans
                    .keys()
                    .copied()
                    .collect::<Vec<_>>(),
                [LoanId(0)]
            );
            assert_eq!(graph.authority[last].opaque, FALSE);
        },
    );
}

#[test]
pub(crate) fn normalized_slot_regions_keep_canonical_root_and_original_view() {
    inspect("r:{->n:=1;p:&n;v:*p;n=2}", |graph, reach| {
        let value = reads(graph, reach)[0];
        let read = graph
            .nodes
            .iter()
            .filter_map(|node| node.access.as_ref())
            .find(|access| access.kind == Kind::Read && access.target == Target::Pointee(value))
            .unwrap();
        let write = graph
            .nodes
            .iter()
            .filter_map(|node| node.access.as_ref())
            .find(|access| access.kind == Kind::Write)
            .unwrap();
        assert!(matches!(read.regions[0].source, Source::Slot { .. }));
        assert_eq!(read.regions[0].source, write.regions[0].source);
    });
}

#[test]
pub(crate) fn input_authority_is_distinct_from_the_parameter_cell() {
    inspect_body(
        "id<&int32>:(p<&int32>){q:&*p;->q}",
        Some(0),
        |graph, reach| {
            graph.solve_authority(reach).unwrap();
            assert_eq!(graph.loans.len(), 2);
            assert!(matches!(
                graph.values[graph.loans[0].value].origins[0].source,
                Source::Input { .. }
            ));
            assert_eq!(
                graph.loans[1]
                    .parents
                    .loans
                    .keys()
                    .copied()
                    .collect::<Vec<_>>(),
                [LoanId(0)]
            );
            let regions = graph
                .nodes
                .iter()
                .filter_map(|node| node.access.as_ref())
                .flat_map(|access| &access.regions)
                .collect::<Vec<_>>();
            assert!(
                regions
                    .iter()
                    .any(|region| matches!(region.source, Source::Local { id: 0, .. }))
            );
            assert!(
                regions
                    .iter()
                    .any(|region| matches!(region.source, Source::Input { id: 0, .. }))
            );
        },
    );
}

#[test]
pub(crate) fn authority_growth_and_invalid_parent_graphs_fail_without_publication() {
    inspect("a:1;p:&a;v:*p", |graph, _| {
        let mut node = Node::default();
        let count = graph.loans.len();
        graph.origins = crate::loans::MAX_ORIGINS;
        assert!(graph.copy_link(&mut node, 0, 0, TRUE).is_err());
        assert!(node.copies.is_empty());
        assert!(
            graph
                .grant_mode(0, 0, None, crate::hir::ReferenceMode::Shared)
                .is_err()
        );
        assert_eq!(graph.loans.len(), count);
        let source = Authority {
            loans: [(LoanId(0), TRUE)].into(),
            opaque: FALSE,
        };
        let mut target = Authority::default();
        assert!(graph.merge_authority(&mut target, &source, TRUE).is_err());
        assert!(target.loans.is_empty());
        graph.origins = 0;
        graph.loans[0].parents.loans.insert(LoanId(0), TRUE);
        assert!(graph.check_parents().is_err());
        graph.loans[0].parents.loans.clear();
        graph.loans[0].parents.loans.insert(LoanId(count), TRUE);
        assert!(graph.check_parents().is_err());
    });
}
