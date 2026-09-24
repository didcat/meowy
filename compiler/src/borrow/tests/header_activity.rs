use super::{accepts, rejects};

#[test]
pub(crate) fn empty_and_full_restart_predecessors_keep_their_actual_payloads() {
    for source in [
        "<C>:<{view<&int32><null>}>;empty<C>:{};p:=&empty;count:=0;'loop{p=&empty;count=count+1;|count<2|'loop.restart()};copy:*p;|copy.view<&int32>|value:*(copy.view<&int32>)",
        "<C>:<{view<&int32><null>}>;a:=1;full<C>:{->view:&a};empty<C>:{};p:=&empty;count:=0;'loop{count=count+1;|count<2|{p=&full;'loop.restart()};p=&empty};a=2;copy:*p;|copy.view<&int32>|value:*(copy.view<&int32>)",
        "<V>:<&int32><null>;empty<V>:null;p:=&empty;count:=0;'loop{p=&empty;count=count+1;|count<2|'loop.restart()};copy:*p",
    ] {
        accepts(source);
    }
    rejects(
        "<C>:<{view<&int32><null>}>;a:=1;full<C>:{->view:&a};empty<C>:{};p:=&empty;count:=0;'loop{copy:*p;|copy.view<null>|{a=2;p=&full};|copy.view<&int32>|value:*(copy.view<&int32>);count=count+1;|count<2|'loop.restart()}",
        "E302",
    );
}

#[test]
pub(crate) fn nested_restart_activity_stays_under_its_outer_variant() {
    let types = "<V>:<&int32><null>;<A>:<{flag<boolean>;view<V>}>;<B>:<{n<int32>;view<V>}>;<C>:<{item<A><B>}>;";
    let values = "a:=1;first<C>:{->item:{->flag:true;->view:&a}};second<C>:{->item:{->n:2}};p:=&first;count:=0;";
    accepts(&format!(
        "{types}{values}'loop{{copy:*p;|copy.item<B>|a=3;|copy.item<A>|{{item:copy.item<A>;|item.view<&int32>|value:*(item.view<&int32>)}};p=&second;count=count+1;|count<2|'loop.restart()}}"
    ));
    rejects(
        &format!(
            "{types}{values}'loop{{copy:*p;|copy.item<A>|{{item:copy.item<A>;a=3;|item.view<&int32>|value:*(item.view<&int32>)}};p=&second;count=count+1;|count<2|'loop.restart()}}"
        ),
        "E302",
    );
}

#[test]
pub(crate) fn inactive_header_paths_do_not_create_bounds_but_active_paths_keep_them() {
    let prefix = "<C>:<{view<&int32><null>}>;first<&int32>:(p<&int32>,other<&string>){->p};a:1;other:=\"old\";full<C>:{->view:first(&a,&other)};empty<C>:{};p:=&empty;count:=0;";
    accepts(&format!(
        "{prefix}'loop{{p=&empty;count=count+1;|count<2|'loop.restart()}};other=\"new\";copy:*p;|copy.view<&int32>|value:*(copy.view<&int32>)"
    ));
    rejects(
        &format!(
            "{prefix}'loop{{copy:*p;|copy.view<null>|{{other=\"new\";p=&full}};|copy.view<&int32>|value:*(copy.view<&int32>);count=count+1;|count<2|'loop.restart()}}"
        ),
        "E302",
    );
    accepts(
        "<C>:<{view<&int32><null>}>;full<C>:(&1).{->view:$};p:=&full;'loop{p=&full;'loop.restart()}",
    );
    rejects(
        "<C>:<{view<&int32><null>}>;full<C>:(&1).{->view:$};p:=&full;'loop{copy:*p;p=&full;'loop.restart()}",
        "E303",
    );
}

#[test]
pub(crate) fn fixed_point_identity_includes_activity_without_source_changes() {
    use crate::borrow::{BTreeMap, Origin, Source, State, Step};
    use crate::borrow_value::Active;
    use crate::flow::{Flow, TRUE};
    use crate::hir::{Field, Type};
    let integer = Type::Int {
        bits: 32,
        signed: true,
    };
    let ty = Type::Reference(Box::new(Type::Record {
        primary: Box::new(Type::Null),
        fields: vec![
            Field {
                name: "tag".into(),
                ty: Type::union([Type::Null, integer.clone()]),
                mutable: false,
            },
            Field {
                name: "view".into(),
                ty: Type::Reference(Box::new(integer)),
                mutable: false,
            },
        ],
    }));
    let tag = vec![Step::Deref, Step::Slot(1)];
    let a = State {
        origins: vec![
            Origin {
                component: Vec::new(),
                source: Source::Local {
                    id: 0,
                    fields: Vec::new(),
                },
                guard: TRUE,
            },
            Origin {
                component: vec![Step::Deref, Step::Slot(2)],
                source: Source::Local {
                    id: 1,
                    fields: Vec::new(),
                },
                guard: TRUE,
            },
        ],
        active: vec![Active {
            component: tag.clone(),
            member: 1,
            guard: TRUE,
        }],
        ..State::default()
    };
    let mut flow = Flow::new();
    let choice = flow.fresh();
    let mut b = a.clone();
    b.active = vec![
        Active {
            component: tag.clone(),
            member: 0,
            guard: choice,
        },
        Active {
            component: tag,
            member: 1,
            guard: flow.not(choice),
        },
    ];
    let span = crate::ast::Span::default();
    let shape = crate::borrow::header::Shape::new(&ty, &mut flow, span).unwrap();
    shape.validate(&a, true, &mut flow, span).unwrap();
    shape.validate(&b, true, &mut flow, span).unwrap();
    let header = |state| BTreeMap::from([(0, BTreeMap::from([(2, state)]))]);
    assert!(!crate::borrow::restart::same(&header(a), &header(b), &mut flow, span).unwrap());
}
