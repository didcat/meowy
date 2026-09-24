use super::access::inspect_body;
use crate::flow::{FALSE, Guard};
use crate::loans::storage::{Event, EventKind, Key, ScopeKind};
use crate::loans::{Graph, Node};

pub(crate) fn inspect(source: &str, names: &[&str], check: impl FnOnce(&mut Graph<'_>, &[Guard])) {
    inspect_body(source, None, |graph, reach| {
        let ids = graph
            .nodes
            .iter()
            .flat_map(|node| &node.events)
            .filter_map(|event| match event.kind {
                EventKind::Use { id, .. }
                    if names.contains(&&source[event.span.start..event.span.end]) =>
                {
                    Some(id)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(names.is_empty() || !ids.is_empty());
        for id in ids {
            graph.stores.get_mut(&id).unwrap().copy = false;
        }
        check(graph, reach);
    });
}

pub(crate) fn accepts(source: &str, names: &[&str]) {
    inspect(source, names, |graph, reach| {
        graph.solve_init(reach).unwrap()
    });
}

pub(crate) fn rejects(source: &str, names: &[&str], code: &str) {
    inspect(source, names, |graph, reach| {
        let error = graph.solve_init(reach).unwrap_err();
        assert_eq!(error.code, code, "{source}: {error:?}");
        assert!(error.span.start < error.span.end);
    });
}

#[test]
pub(crate) fn initialization_is_distinct_from_reference_definitions() {
    inspect("a:1;p:&a;q:p;v:*q", &[], |graph, reach| {
        for node in &mut graph.nodes {
            node.defs.clear();
        }
        graph.solve_init(reach).unwrap();
        assert_eq!(graph.stores.len(), 4);
    });
    inspect("a:1;p:&a;v:*p", &[], |graph, reach| {
        for node in &mut graph.nodes {
            node.events.retain(|event| event.kind != EventKind::Init(0));
        }
        assert_eq!(graph.solve_init(reach).unwrap_err().code, "E309");
    });
}

#[test]
pub(crate) fn returning_rhs_commits_initialization_after_completed_effects() {
    inspect("a:=1;b:=2;a={b=3;->b};v:a", &[], |graph, reach| {
        graph.solve_init(reach).unwrap();
        let events = graph
            .nodes
            .iter()
            .zip(reach)
            .filter(|(_, guard)| **guard != FALSE)
            .flat_map(|(node, _)| &node.events)
            .filter_map(|event| match event.kind {
                EventKind::Init(id) => Some((false, id)),
                EventKind::Use { id, .. } => Some((true, id)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            events,
            [
                (false, 0),
                (false, 1),
                (false, 1),
                (true, 1),
                (false, 0),
                (true, 0),
                (false, 2)
            ]
        );
    });
}

#[test]
pub(crate) fn copy_takes_do_not_move_and_inspection_does_not_take() {
    accepts("a:1;p:&a;q:p;v:*p;w:*q", &[]);
    accepts("a:1;p:&a;v:*p;w:*p;s:&*p;z:*s", &["p"]);
    accepts("a:1;p:&a;cell:&p;v:*p", &["p"]);
    rejects("a:1;p:&a;q:p;v:*p", &["p", "q"], "E301");
}

#[test]
pub(crate) fn discarded_and_wrapped_value_takes_preserve_consumption() {
    for source in [
        "a:1;p:&a;p;v:*p",
        "a:1;p:&a;q:(p);v:*p",
        "a:1;p:&a;q:p<&int32>;v:*p",
    ] {
        rejects(source, &["p"], "E301");
    }
}

#[test]
pub(crate) fn moved_cells_can_be_reinitialized_without_reading_the_old_value() {
    accepts("a:1;b:2;p:=&a;q:p;p=&b;v:*p;w:*q", &["p", "q"]);
    accepts(
        "a:1;b:2;p:=&a;'out{q:p;v:*q;p=&b;'out.leave()};v:*p",
        &["p", "q"],
    );
}

#[test]
pub(crate) fn branch_joins_distinguish_possible_moves_from_guarded_safe_uses() {
    rejects(
        "a:1;p:&a;flag:=true;|flag|{q:p;v:*q};v:*p",
        &["p", "q"],
        "E309",
    );
    accepts(
        "a:1;p:&a;flag:=true;|flag|{q:p;v:*q};|!flag|v:*p",
        &["p", "q"],
    );
    accepts(
        "a:1;b:2;p:=&a;flag:=true;|flag|{q:p;v:*q;p=&b};v:*p",
        &["p", "q"],
    );
}

#[test]
pub(crate) fn short_circuits_keep_skipped_consumption_out_of_availability() {
    rejects(
        "a:1;p:&a;flag:=true;v:flag&&{q:p;->*q==1};w:*p",
        &["p", "q"],
        "E309",
    );
    accepts("a:1;p:&a;v:false&&{q:p;->*q==1};w:*p", &["p", "q"]);
    accepts("a:1;p:&a;v:true||{q:p;->*q==1};w:*p", &["p", "q"]);
}

#[test]
pub(crate) fn named_leaves_keep_completed_moves_and_skip_unfinished_reinitialization() {
    rejects(
        "a:1;b:2;p:=&a;'out{q:p;'out.leave();p=&b};v:*p",
        &["p", "q"],
        "E301",
    );
    rejects(
        "a:1;b:2;p:=&a;flag:=true;'out{|flag|{q:p;'out.leave()};p=&b};v:*p",
        &["p", "q"],
        "E309",
    );
    rejects(
        "a:1;b:2;p:=&a;'out{p={q:p;'out.leave();->&b}};v:*p",
        &["p", "q"],
        "E301",
    );
}

#[test]
pub(crate) fn statement_temporaries_do_not_own_the_surviving_reference_holder() {
    inspect(
        "a:1;p:=&a;(&2).{p=$;v:*p};p=&a;v:*p",
        &[],
        |graph, reach| {
            graph.solve_init(reach).unwrap();
            for (id, statement) in &graph.proofs.temporaries {
                let store = graph.stores.get(id).unwrap();
                assert_eq!(
                    graph.scopes[store.scope.0].kind,
                    ScopeKind::Statement(*statement)
                );
            }
            let holder = &graph.stores[&1];
            assert!(matches!(
                graph.scopes[holder.scope.0].kind,
                ScopeKind::Block(_)
            ));
        },
    );
    accepts(
        "value:(&7).{p:=$;i:=0;'inner{v:*p;p=$;i=i+1;|i<2|'inner.restart()};->*p}",
        &[],
    );
}

#[test]
pub(crate) fn canonical_emitted_cells_belong_to_their_target_scope() {
    inspect(
        "a:1;p:=&a;r:'out{{'out->n:=7;p=&n};v:*p;p=&a};w:*p",
        &[],
        |graph, reach| {
            graph.solve_init(reach).unwrap();
            for alias in graph
                .proofs
                .aliases
                .values()
                .filter(|alias| alias.field == "n")
            {
                let store = &graph.stores[&alias.root];
                assert_eq!(
                    graph.scopes[store.scope.0].kind,
                    ScopeKind::Block(alias.target)
                );
            }
        },
    );
    accepts(
        "flag:=true;r:'out{|flag|{'out->n:=1;v:n;n=3};|!flag|{'out->n:=2;v:n;n=4}}",
        &[],
    );
}

#[test]
pub(crate) fn restart_reinitializes_local_cells_but_not_moved_ancestor_holders() {
    accepts(
        "i:=0;'loop{a:1;p:&a;q:p;v:*q;i=i+1;|i<2|'loop.restart()}",
        &["p", "q"],
    );
    rejects(
        "a:1;p:&a;i:=0;'loop{q:p;v:*q;i=i+1;|i<2|'loop.restart()}",
        &["p", "q"],
        "E309",
    );
    accepts(
        "a:1;p:=&a;i:=0;'loop{p=&a;q:p;v:*q;i=i+1;|i<2|'loop.restart()}",
        &["p", "q"],
    );
}

#[test]
pub(crate) fn panic_ends_active_scopes_and_parameter_cells_have_function_ownership() {
    inspect_body(
        "d:@\"debug\";f<null>:(p<int32>){v:p;d.panic(\"stop\")}",
        Some(0),
        |graph, reach| {
            graph.solve_init(reach).unwrap();
            assert_eq!(
                graph.scopes[graph.stores[&0].scope.0].kind,
                ScopeKind::Function
            );
            let ends = graph
                .nodes
                .iter()
                .zip(reach)
                .filter(|(_, guard)| **guard != FALSE)
                .flat_map(|(node, _)| &node.events)
                .filter_map(|event| match event.kind {
                    EventKind::End(scope) => Some(graph.scopes[scope.0].kind),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(matches!(
                ends.as_slice(),
                [ScopeKind::Block(_), ScopeKind::Function]
            ));
        },
    );
}

#[test]
pub(crate) fn ended_storage_cannot_be_read_or_reinitialized_without_scope_entry() {
    for kind in [EventKind::Use { id: 0, take: false }, EventKind::Init(0)] {
        inspect("a:1;v:a", &[], |graph, reach| {
            let node = *graph.current.last().unwrap();
            graph.nodes[node].events.push(Event {
                kind,
                span: crate::ast::Span { start: 6, end: 7 },
            });
            assert_eq!(graph.solve_init(reach).unwrap_err().code, "E309");
        });
    }
}

#[test]
pub(crate) fn unused_cells_do_not_fill_every_availability_snapshot() {
    let source = (0..256)
        .map(|id| format!("v{id}:{id};"))
        .collect::<String>();
    inspect(&source, &[], |graph, reach| {
        graph.solve_init(reach).unwrap();
        assert_eq!(graph.stores.len(), 256);
        assert!(
            graph
                .availability
                .iter()
                .flat_map(|values| values.keys())
                .all(|key| matches!(key, Key::Scope(_)))
        );
        assert!(
            graph
                .availability
                .iter()
                .flat_map(|values| values.keys())
                .all(|key| {
                    matches!(key, Key::Scope(scope) if graph.scopes[scope.0].cells != 0)
                })
        );
    });
}

#[test]
pub(crate) fn lifecycle_and_solver_budgets_stop_before_publishing_new_state() {
    inspect("a:1;v:a", &[], |graph, reach| {
        let mut node = Node::default();
        graph.origins = crate::loans::MAX_ORIGINS;
        assert!(
            graph
                .event(&mut node, EventKind::Init(0), crate::ast::Span::default())
                .is_err()
        );
        assert!(node.events.is_empty());
        graph.work = crate::loans::MAX_WORK;
        assert!(graph.solve_init(reach).is_err());
        assert!(graph.availability.is_empty());
    });
}
