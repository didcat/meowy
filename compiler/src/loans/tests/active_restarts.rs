use super::{accepts, rejects};

#[test]
pub(crate) fn active_restart_headers_allow_initial_and_backedge_absent_paths() {
    accepts(
        "<C>:<{view<&int32><null>}>;a:1;empty<C>:{};full<C>:{->view:&a};p:=&empty;i:=0;'again{seen:p.view;|seen<&int32>|v:*(seen~<&int32>);p=&full;i=i+1;|i<2|'again.restart()};last:p.view;|last<&int32>|w:*(last~<&int32>)",
    );
    accepts(
        "<C>:<{view<&int32><null>}>;a:=1;empty<C>:{};full<C>:{->view:&a};p:=&full;i:=0;'again{seen:p.view;|seen<&int32>|v:*(seen~<&int32>);p=&empty;i=i+1;|i<2|'again.restart()};a=2;last:p.view;|last<null>|done:true",
    );
    accepts(
        "cell<&int32><null>:null;p:=&cell;i:=0;'again{seen:*p;|seen<null>|done:true;p=&cell;i=i+1;|i<2|'again.restart()}",
    );
}

#[test]
pub(crate) fn restart_demand_resets_before_predecessor_variant_filtering() {
    rejects(
        "<C>:<{view<&int32><null>}>;a:=1;empty<C>:{};full<C>:{->view:&a};p:=&empty;i:=0;'again{seen:p.view;|seen<null>|a=2;|seen<&int32>|v:*(seen~<&int32>);p=&full;i=i+1;|i<2|'again.restart()}",
        "E302",
    );
    rejects(
        "cell<&int32><null>:null;a:=1;full<&int32><null>:&a;p:=&cell;i:=0;'again{seen:*p;|seen<null>|a=2;|seen<&int32>|v:*(seen~<&int32>);p=&full;i=i+1;|i<2|'again.restart()}",
        "E302",
    );
    accepts(
        "<C>:<{view<&int32><null>}>;a:=1;empty<C>:{};full<C>:{->view:&a};p:=&full;i:=0;'again{p=&empty;a=2;seen:p.view;|seen<null>|done:true;i=i+1;|i<2|'again.restart()}",
    );
    accepts(
        "<C>:<{view<&int32><null>}>;a:=1;empty<C>:{};p:=&empty;a=2;full<C>:{->view:&a};i:=0;'again{seen:p.view;|seen<&int32>|v:*(seen~<&int32>);p=&full;i=i+1;|i<2|'again.restart()}",
    );
}

#[test]
pub(crate) fn stored_activity_does_not_add_header_reads_or_retarget_old_copies() {
    accepts(
        "<C>:<{view<&int32><null>}>;a:=1;empty<C>:{};full<C>:{->view:&a};p:=&full;i:=0;'again{a=2;same:p==p;p=&empty;i=i+1;|i<2|'again.restart()}",
    );
    accepts(
        "<C>:<{view<&int32><null>}>;a:=1;empty<C>:{};full<C>:{->view:&a};p:=&full;i:=0;'again{a=2;|p.view<null>|seen:true;p=&empty;i=i+1;|i<2|'again.restart()}",
    );
    rejects(
        "<C>:<{view<&int32><null>}>;a:=1;empty<C>:{};full<C>:{->view:&a};p:=&full;old:p.view;i:=0;'again{p=&empty;i=i+1;|i<2|'again.restart()};a=2;|old<&int32>|v:*(old~<&int32>)",
        "E302",
    );
    rejects(
        "<C>:<{view<&int32><null>}>;a:1;empty<C>:{};full<C>:{->view:&a};p:=&full;cell:&p;i:=0;'again{p=&empty;i=i+1;|i<2|'again.restart()};same:cell==&p",
        "E302",
    );
}

#[test]
pub(crate) fn active_header_components_keep_nested_public_call_bounds() {
    rejects(
        "<C>:<{view<&int32><null>}>;id<&C>:(p<&C>,s<&string>){->p};a:1;full<C>:{->view:&a};s:=\"old\";p:=id(&full,&s);i:=0;'again{copy:*p;s=\"new\";seen:copy.view;|seen<&int32>|v:*(seen~<&int32>);p=&full;i=i+1;|i<2|'again.restart()}",
        "E302",
    );
    accepts(
        "<C>:<{view<&int32><null>}>;copy<C>:(p<&C>,s<&string>){->*p};empty<C>:{};s:=\"old\";p:=&empty;i:=0;'again{out:copy(p,&s);s=\"new\";|out.view<null>|seen:true;p=&empty;i=i+1;|i<2|'again.restart()}",
    );
    accepts(
        "<C>:<{view<&int32><null>}>;a:1;full<C>:{->view:&a};empty<C>:{};p:=&empty;i:=0;j:=0;'outer{j=0;'inner{seen:p.view;|seen<&int32>|v:*(seen~<&int32>);p=&full;j=j+1;|j<2|'inner.restart()};p=&empty;i=i+1;|i<2|'outer.restart()}",
    );
}

#[test]
pub(crate) fn nested_variant_paths_require_their_parent_activity() {
    accepts(
        "<A>:<{view<&int32><null>;a<boolean>}>;<B>:<{view<&string><null>;b<boolean>}>;<U>:<A><B>;n:1;full<U>:{->view:&n;->a:true};empty<U>:{->view:null;->b:true};p:=&empty;i:=0;'again{row:*p;|row<A>|{seen:row~<A>.view;|seen<&int32>|v:*(seen~<&int32>)};|row<B>|{seen:row~<B>.view;|seen<null>|done:true};p=&full;i=i+1;|i<2|'again.restart()}",
    );
    rejects(
        "<A>:<{view<&int32><null>;a<boolean>}>;<B>:<{view<&string><null>;b<boolean>}>;<U>:<A><B>;n:=1;full<U>:{->view:&n;->a:true};empty<U>:{->view:null;->b:true};p:=&empty;i:=0;'again{n=2;row:*p;|row<A>|{seen:row~<A>.view;|seen<&int32>|v:*(seen~<&int32>)};p=&full;i=i+1;|i<2|'again.restart()}",
        "E302",
    );
    accepts(
        "a:1;holder:{->tag<int32><null>:null;->view<&int32><null>:&a};p:=&holder;i:=0;'again{copy:*p;|copy.tag<null>|done:true;p=&holder;i=i+1;|i<2|'again.restart()}",
    );
}

#[test]
pub(crate) fn active_headers_keep_unreachable_transfer_and_lifetime_boundaries() {
    accepts(
        "<C>:<{view<&int32><null>}>;d:@\"debug\";stop<&C>:(){d.panic(\"stop\")};run<null>:(flag<boolean>){a:1;full<C>:{->view:&a};p:=&full;i:=0;'again{|flag|{p=stop();'again.restart()};p=&full;i=i+1;|i<2|'again.restart()}}",
    );
    rejects(
        "cell:&1;optional<&int32><null>:cell;p:=&optional;'again{p=&optional;'again.restart()}",
        "E303",
    );
    accepts(
        "a:1;empty<&int32><null>:null;p:=&empty;'again{local<&int32><null>:&a;p=&local;'again.restart()}",
    );
    accepts(
        "<C>:<{view<&int32><null>}>;empty<C>:{};p:=&empty;'again{p=&{->view<&int32><null>:null};'again.restart()}",
    );
}

#[test]
pub(crate) fn missing_optional_transfers_require_predecessor_proof() {
    use crate::borrow::state::Predecessor;
    use crate::borrow::{Facts, Origin, Proofs, Source, State, Step};
    use crate::borrow_value::Active;
    use crate::flow::{FALSE, Flow, TRUE};
    use crate::hir::{Block, Program, Type};
    use crate::loans::branches::Versions;
    use crate::loans::{Bundle, Graph};
    use std::collections::BTreeMap;

    let int = Type::Int {
        bits: 32,
        signed: true,
    };
    let reference = Type::Reference(Box::new(int.clone()));
    let optional = Type::union([Type::Null, reference.clone()]);
    let member = optional
        .members()
        .iter()
        .position(|ty| ty == &reference)
        .unwrap();
    let null = optional
        .members()
        .iter()
        .position(|ty| ty == &Type::Null)
        .unwrap();
    let path = vec![Step::Deref, Step::Variant(member)];
    let mut flow = Flow::new();
    let present = flow.fresh();
    let future = flow.fresh();
    let origin = |component, id, guard| Origin {
        component,
        source: Source::Local {
            id,
            fields: Vec::new(),
        },
        guard,
    };
    let state = State {
        origins: vec![
            origin(Vec::new(), 1, TRUE),
            origin(path.clone(), 2, present),
        ],
        active: vec![
            Active {
                component: vec![Step::Deref],
                member,
                guard: present,
            },
            Active {
                component: vec![Step::Deref],
                member: null,
                guard: flow.not(present),
            },
        ],
        ..State::default()
    };
    let program = Program {
        body: Block {
            id: 0,
            stmts: Vec::new(),
            ty: Type::Null,
        },
        functions: Vec::new(),
        locals: vec![Type::Reference(Box::new(optional.clone())), optional, int],
    };
    let facts = Facts::default();
    let proofs = Proofs::default();
    let mut graph = Graph::new(&program, &facts, &proofs, &mut flow);
    let root = graph.value(vec![origin(Vec::new(), 1, TRUE)]).unwrap();
    let leaf = graph.value(vec![origin(path.clone(), 2, present)]).unwrap();
    let next_root = graph.value(vec![origin(Vec::new(), 1, TRUE)]).unwrap();
    let next_leaf = graph.value(vec![origin(path.clone(), 2, future)]).unwrap();
    let partial = Versions::from([(0, Bundle::from([(Vec::new(), root)]))]);
    let full = Versions::from([(0, Bundle::from([(Vec::new(), root), (path.clone(), leaf)]))]);
    let target = Versions::from([(
        0,
        Bundle::from([(Vec::new(), next_root), (path.clone(), next_leaf)]),
    )]);
    let mut proof = Predecessor {
        values: BTreeMap::from([(0, state)]),
        entered: TRUE,
    };

    let (node, missing) = graph
        .header_transfer(&partial, &target, Some(&proof))
        .unwrap();
    assert_eq!(missing, present);
    assert!(node.uses.is_empty());
    assert_eq!(node.defs.len(), 2);
    assert!(graph.guards.overlap(missing, graph.guards.not(future)));

    let (node, missing) = graph.header_transfer(&full, &target, Some(&proof)).unwrap();
    assert_eq!(missing, FALSE);
    assert!(node.transfers.contains(&(next_leaf, leaf, present)));
    assert!(node.uses.is_empty());

    proof.entered = graph.guards.not(present);
    let (node, missing) = graph
        .header_transfer(&partial, &target, Some(&proof))
        .unwrap();
    assert_eq!(missing, FALSE);
    assert_eq!(node.transfers.len(), 1);
    assert!(node.uses.is_empty());
    let (_, missing) = graph.header_transfer(&partial, &target, None).unwrap();
    assert_eq!(missing, TRUE);

    let truncated = Versions::from([(0, Bundle::from([(Vec::new(), next_root)]))]);
    proof.entered = TRUE;
    let (_, missing) = graph
        .header_transfer(&full, &truncated, Some(&proof))
        .unwrap();
    assert_eq!(missing, present);
    proof.entered = graph.guards.not(present);
    let (_, missing) = graph
        .header_transfer(&full, &truncated, Some(&proof))
        .unwrap();
    assert_eq!(missing, FALSE);
}
