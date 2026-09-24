use crate::borrow::Source;
use crate::flow::{FALSE, Guard, TRUE};
use crate::hir::Program;
use crate::loans::access::{Kind, Target};
use crate::loans::{Graph, Path, Step};

pub(crate) fn inspect(source: &str, check: impl FnOnce(&mut Graph<'_>, &[Guard])) {
    inspect_body(source, None, check);
}

pub(crate) fn inspect_body(
    source: &str,
    function: Option<usize>,
    check: impl FnOnce(&mut Graph<'_>, &[Guard]),
) {
    let tree = crate::parser::parse(source).unwrap();
    let mut checker = crate::check::Checker::new();
    let body = checker.block(&tree, None, None).unwrap();
    let program = Program {
        body,
        functions: checker.functions.into_iter().flatten().collect(),
        locals: checker.locals,
    };
    checker.proofs.conditions = checker.guards;
    checker.proofs.tags = checker.tags;
    let facts = crate::borrow::check(&program, &mut checker.flow, &checker.proofs).unwrap();
    let mut graph = Graph::new(&program, &facts, &checker.proofs, &mut checker.flow);
    let (body, params) = if let Some(id) = function {
        let function = &program.functions[id];
        (&function.body, function.params.as_slice())
    } else {
        (&program.body, &[][..])
    };
    graph.build(body, params).unwrap();
    let reach = graph.reach().unwrap();
    graph.access_evidence(&reach).unwrap();
    check(&mut graph, &reach);
}

pub(crate) fn trace(graph: &Graph<'_>, reach: &[Guard]) -> Vec<(Kind, Target, Path)> {
    graph
        .nodes
        .iter()
        .zip(reach)
        .filter(|(_, guard)| **guard != FALSE)
        .filter_map(|(node, _)| {
            node.access
                .as_ref()
                .map(|access| (access.kind, access.target.clone(), access.path.clone()))
        })
        .collect()
}

#[test]
pub(crate) fn owner_reads_precede_the_returning_store_without_extending_loans() {
    inspect("x:=1;y:=2;x={v:y;->x+y};v:x", |graph, reach| {
        let events = trace(graph, reach);
        let kinds = events.iter().map(|event| event.0).collect::<Vec<_>>();
        assert_eq!(
            kinds,
            [Kind::Read, Kind::Read, Kind::Read, Kind::Write, Kind::Read]
        );
        let roots = events
            .iter()
            .map(|(_, target, _)| match target {
                Target::Storage { place, .. } => place.root,
                _ => panic!("expected direct storage"),
            })
            .collect::<Vec<_>>();
        assert_eq!(roots, [1, 0, 1, 0, 0]);
        for node in &graph.nodes {
            if node
                .access
                .as_ref()
                .is_some_and(|access| access.kind == Kind::Read)
            {
                assert!(node.uses.is_empty());
                assert!(node.defs.is_empty());
                assert!(node.transfers.is_empty());
            }
        }
    });
    super::accepts("x:=1;p:&x;x=*p+1");
    super::rejects("x:=1;p:&x;x=*p+1;v:*p", "E302");
}

#[test]
pub(crate) fn direct_paths_distinguish_fields_primary_tags_and_whole_values() {
    inspect(
        "r:={->1;->n<int32><null>:null};v<int32>:r;|r.n<null>|{};all:r",
        |graph, reach| {
            let events = trace(graph, reach);
            let reads = events
                .iter()
                .filter(|event| event.0 == Kind::Read)
                .collect::<Vec<_>>();
            assert!(reads.iter().any(|event| event.2 == [Step::Slot(0)]));
            assert!(reads.iter().any(|event| event.2.is_empty()));
            assert!(
                events
                    .iter()
                    .any(|event| event.0 == Kind::Tag && event.2 == [Step::Slot(1)])
            );
        },
    );
    inspect("r:={->a:=1;->b:=2};v:r.a;r.b=3;all:r", |graph, reach| {
        let events = trace(graph, reach);
        let selected = events
            .iter()
            .find(|event| event.0 == Kind::Read && event.2 == [Step::Slot(1)])
            .unwrap();
        let Target::Storage { place, .. } = &selected.1 else {
            panic!("direct field")
        };
        let root = place.root;
        assert!(events.iter().any(|event| event.0 == Kind::Write
            && matches!(&event.1, Target::Storage { place, .. } if place.root == root && place.fields == [1])));
        assert!(events.iter().any(|event| event.0 == Kind::Read
            && event.2.is_empty()
            && matches!(&event.1, Target::Storage { place, .. } if place.root == root)));
    });
}

#[test]
pub(crate) fn narrowed_tag_paths_keep_the_original_variant_storage() {
    inspect(
        "<A>:<{n<int32><null>}>;<B>:<{s<string>}>;u<A><B>:={->n<int32><null>:null};|u<A>|{|u.n<null>|{}}",
        |graph, reach| {
            let events = trace(graph, reach);
            assert!(
                events
                    .iter()
                    .any(|event| event.0 == Kind::Tag && event.2.is_empty())
            );
            assert!(events.iter().any(|event| event.0 == Kind::Tag
                && matches!(event.2.as_slice(), [Step::Variant(_), Step::Slot(1)])));
        },
    );
}

#[test]
pub(crate) fn pointee_reads_keep_versions_and_never_resolve_public_bounds_as_sources() {
    inspect(
        "first<&int32>:(p<&int32>,q<&int32>){->p};a:=1;b:=2;p:=first(&a,&b);v:*p;p=&b;w:*p",
        |graph, reach| {
            let ids = trace(graph, reach)
                .into_iter()
                .filter_map(|(kind, target, _)| match target {
                    Target::Pointee(id) if kind == Kind::Read => Some(id),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(ids.len(), 4);
            let ids = &ids[2..];
            assert_ne!(ids[0], ids[1]);
            assert!(
                graph.values[ids[0]]
                    .iter()
                    .any(|origin| matches!(origin.source, Source::Local { id: 2, .. }))
            );
            assert!(graph.values[ids[0]].len() > graph.values[ids[1]].len());
            assert!(
                graph.values[ids[1]]
                    .iter()
                    .all(|origin| matches!(origin.source, Source::Local { id: 3, .. }))
            );
        },
    );
}

#[test]
pub(crate) fn shared_acquisitions_and_pointee_tags_keep_source_spans_and_no_payload_use() {
    let source = "cell<&int32><null>:null;p:=&cell;|(*p)<null>|{}";
    inspect(source, |graph, reach| {
        let events = trace(graph, reach);
        assert!(events.iter().any(|event| event.0 == Kind::Borrow));
        assert!(
            events
                .iter()
                .any(|event| event.0 == Kind::Tag && matches!(event.1, Target::Pointee(_)))
        );
        for node in &graph.nodes {
            if let Some(access) = &node.access {
                assert!(access.span.start < access.span.end);
                assert!(access.span.end <= source.len());
                if access.kind == Kind::Borrow {
                    assert!(node.uses.is_empty());
                }
            }
        }
    });
}

#[test]
pub(crate) fn alias_reads_and_writes_share_canonical_roots_but_retain_views() {
    inspect(
        "flag:=true;r:'out{|flag|{'out->n:=1;v:n;n=3};|!flag|{'out->n:=2;v:n;n=4}}",
        |graph, reach| {
            let aliases = graph
                .proofs
                .aliases
                .iter()
                .filter(|(_, alias)| alias.field == "n")
                .collect::<Vec<_>>();
            assert_eq!(aliases.len(), 2);
            assert_eq!(aliases[0].1.root, aliases[1].1.root);
            let events = trace(graph, reach);
            for (view, alias) in aliases {
                for kind in [Kind::Read, Kind::Write] {
                    assert!(events.iter().any(|event| event.0 == kind
                    && matches!(&event.1, Target::Storage { place, view: actual } if actual == view && place.root == alias.root)));
                }
            }
        },
    );
}

#[test]
pub(crate) fn nonreturning_rhs_has_reads_but_no_outer_write_and_indices_keep_regions() {
    inspect(
        "a:=1;b:=2;'out{a={b=3;v:b;'out.leave()}};v:a",
        |graph, reach| {
            let writes = trace(graph, reach)
                .into_iter()
                .filter(|event| event.0 == Kind::Write)
                .collect::<Vec<_>>();
            assert_eq!(writes.len(), 1);
            assert!(matches!(&writes[0].1, Target::Storage { place, .. } if place.root == 1));
        },
    );
    inspect(
        "r:={->items:=[1,2];->other:=3};i:=1;r.items[{i=2;->i}]=r.other",
        |graph, reach| {
            let events = trace(graph, reach);
            let write = events
                .iter()
                .rposition(|event| event.0 == Kind::Write)
                .unwrap();
            assert!(
                matches!(&events[write].1, Target::Storage { place, .. } if place.fields == [0])
            );
            assert!(
                events[..write]
                    .iter()
                    .any(|event| event.0 == Kind::Read && event.2 == [Step::Slot(2)])
            );
            assert!(events[..write].iter().any(|event| event.0 == Kind::Write));
        },
    );
}

#[test]
pub(crate) fn static_type_predicates_do_not_invent_storage_tag_reads() {
    inspect("x:=1;p:&x;|x<int32>|{};|p<&int32>|{}", |graph, reach| {
        let events = trace(graph, reach);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, Kind::Borrow);
    });
}

#[test]
pub(crate) fn access_reach_preserves_complementary_branch_guards() {
    inspect(
        "a:=1;b:=2;flag:=true;|flag|v:a;|!flag|v:b",
        |graph, reach| {
            let guard = |root| {
                graph.nodes.iter().zip(reach).find_map(|(node, guard)| {
                node.access.as_ref().and_then(|access| {
                    (access.kind == Kind::Read
                        && matches!(&access.target, Target::Storage { place, .. } if place.root == root))
                    .then_some(*guard)
                })
            }).unwrap()
            };
            let a = guard(0);
            let b = guard(1);
            assert_ne!(a, FALSE);
            assert_ne!(b, FALSE);
            assert!(!graph.guards.overlap(a, b));
        },
    );
}

#[test]
pub(crate) fn restart_transfers_do_not_synthesize_reads_or_acquisitions() {
    inspect(
        "a:1;b:2;p:=&a;i:=0;'loop{v:*p;p=&b;i=i+1;|i<2|'loop.restart()}",
        |graph, reach| {
            let resets = graph
                .nodes
                .iter()
                .filter(|node| node.next.iter().any(|edge| edge.reset))
                .collect::<Vec<_>>();
            assert!(resets.len() >= 2);
            assert!(resets.iter().all(|node| node.access.is_none()));
            assert_eq!(
                trace(graph, reach)
                    .iter()
                    .filter(|event| event.0 == Kind::Borrow)
                    .count(),
                2
            );
        },
    );
}

#[test]
pub(crate) fn missing_pointee_evidence_is_rejected_only_on_reachable_paths() {
    inspect(
        "p<&int32><null>:null;|p<&int32>|v:*(p~<&int32>)",
        |graph, reach| {
            let id = graph
                .nodes
                .iter()
                .position(|node| {
                    node.access
                        .as_ref()
                        .is_some_and(|access| access.target == Target::Missing)
                })
                .unwrap();
            assert_eq!(reach[id], FALSE);
            let mut reach = reach.to_vec();
            reach[id] = TRUE;
            let error = graph.access_evidence(&reach).unwrap_err();
            assert_eq!(error.code, "B001");
            assert!(error.message.contains("missing pointee"));
        },
    );
}

#[test]
pub(crate) fn access_paths_stop_before_exceeding_storage_or_work_budgets() {
    inspect("x:=1;v:x", |graph, _| {
        let count = graph.nodes.len();
        let target = graph.storage(0, Vec::new());
        graph.origins = crate::loans::MAX_ORIGINS;
        assert!(
            graph
                .access(
                    Kind::Read,
                    target,
                    &[Step::Slot(0)],
                    crate::ast::Span::default()
                )
                .is_err()
        );
        assert_eq!(graph.nodes.len(), count);
        assert_eq!(graph.origins, crate::loans::MAX_ORIGINS);
        graph.origins = 0;
        graph.work = crate::loans::MAX_WORK;
        assert!(
            graph
                .read_local(0, &[], Kind::Read, crate::ast::Span::default(), true)
                .is_err()
        );
        assert_eq!(graph.nodes.len(), count);
    });
}
