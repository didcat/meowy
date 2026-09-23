use super::*;
use crate::flow::{FALSE, TRUE};

pub(crate) fn accepts(source: &str) {
    let result = crate::compile(source);
    assert!(result.is_ok(), "{source}: {result:?}");
}

pub(crate) fn rejects(source: &str, code: &str) {
    let errors = crate::compile(source).unwrap_err();
    assert_eq!(errors[0].code, code, "{source}: {errors:?}");
}

#[test]
pub(crate) fn direct_functions_return_symbolic_inputs_and_carrier_components() {
    accepts("id<&int32>:(p<&int32>){->p};a:1;r:id(&a);v:*r");
    accepts("id:(p<&int32>){->p};a:1;r:id(&a);v:*r");
    accepts(
        "pick<&int32>:(p<{left<&int32>;right<&string>}>){->p.left};a:1;b:\"s\";r:pick({->left:&a;->right:&b});v:*r",
    );
    accepts(
        "copy<{view<&int32>;count<int32>}>:(p<&int32>){->view:p;->count:7};a:1;r:copy(&a);v:*(r.view)",
    );
    accepts("id<&int32><null>:(p<&int32><null>){->p};u:id(null);|u<&int32>|x:*(u<&int32>)");
}

#[test]
pub(crate) fn ignored_and_transitive_inputs_still_bound_returned_views() {
    rejects(
        "first<&int32>:(p<&int32>,q<&string>){->p};wrap<&int32>:(p<&int32>){local:\"s\";->first(p,&local)}",
        "E303",
    );
    rejects(
        "first<&int32>:(p<&int32>,q<&string>){->p};id<&int32>:(p<&int32>){->p};wrap<&int32>:(p<&int32>){local:\"s\";r:first(p,&local);->id(r)}",
        "E303",
    );
    rejects(
        "pick<&int32>:(p<{left<&int32>;right<&string>}>){->p.left};wrap<&int32>:(p<&int32>){local:\"s\";pair:{->left:p;->right:&local};->pick(pair)}",
        "E303",
    );
    accepts(
        "first<&int32>:(p<&int32>,q<&string>){->p};wrap<int32>:(p<&int32>){local:\"s\";->*(first(p,&local))}",
    );
    accepts("first<&int32>:(p<&int32>,q<&string><null>){->p};a:1;r:first(&a,null);x:*r");
}

#[test]
pub(crate) fn every_body_rejects_local_return_origins_even_when_uncalled() {
    rejects("bad<&int32>:(){local:1;->&local}", "E303");
    rejects("bad<&int32>:(p<&int32>){local:1;->&local}", "E303");
    rejects(
        "bad<{view<&int32>}>:(p<&int32>){local:1;->view:&local}",
        "E303",
    );
    rejects(
        "bad<&int32><null>:(flag<boolean>){local:1;|flag|->&local}",
        "E303",
    );
}

#[test]
pub(crate) fn normal_return_proofs_preserve_earlier_leaves_and_separate_definitions() {
    accepts(
        "d:@\"debug\";stop<&int32>:(n<int32>){d.panic(\"stop\")};wrap<int32>:(flag<boolean>) 'out {->*(stop({|flag|{'out->7;'out.leave()};->0}))}",
    );
    accepts("d:@\"debug\";stop<&int32>:(){d.panic(\"stop\")};wrap<&int32>:(){->stop()}");
    accepts("dead<&int32>:(){->dead()}");
    rejects(
        "dead<&int32>:(s<&string>){->dead(s)};bad<&int32>:(flag<boolean>,s<&string>){|flag|dead(s);local:1;->&local}",
        "E303",
    );
    accepts(
        "pick<&int32>:(p<&int32>,n<int32>) 'out {|n==0|{'out->p;'out.leave()};->pick(p,n-1)};a:1;r:pick(&a,2);x:*r",
    );
}

#[test]
pub(crate) fn returned_component_by_input_expansion_has_a_public_source_budget() {
    let fields = (0..64)
        .map(|id| format!("field{id}<&int32>;"))
        .collect::<String>();
    let params = (0..64)
        .map(|id| format!("p{id}<&int32>"))
        .collect::<Vec<_>>()
        .join(",");
    let mut source = format!("make<{{{fields}}}>:({params}){{");
    for id in 0..64 {
        source.push_str(&format!("->field{id}:p0;"));
    }
    source.push_str("};");
    for id in 0..64 {
        source.push_str(&format!("a{id}:{id};"));
    }
    let args = (0..64)
        .map(|id| format!("&a{id}"))
        .collect::<Vec<_>>()
        .join(",");
    source.push_str(&format!("value:make({args})"));
    let errors = crate::compile(&source).unwrap_err();
    assert_eq!(errors[0].code, "B001", "{errors:?}");
    assert!(errors[0].message.contains("budget"));
}

#[test]
pub(crate) fn wide_reference_targets_do_not_expand_an_unbounded_frontier() {
    let fields = (0..4097)
        .map(|id| format!("field{id}<int32>;"))
        .collect::<String>();
    let source = format!(
        "<Wide>:<{{{fields}}}>;id<&Wide>:(p<&Wide>){{->p}};wrap<&Wide>:(p<&Wide>){{->id(p)}}"
    );
    let errors = crate::compile(&source).unwrap_err();
    assert_eq!(errors[0].code, "B001", "{errors:?}");
    assert!(errors[0].message.contains("budget"));
}
#[test]
pub(crate) fn projected_inputs_keep_referent_paths_and_all_input_bounds() {
    accepts(
        r#"<I>:<{n<int32>}>;<R>:<{nested<I>;tag<string>}>;part<&I>:(p<&R>){->&(p.nested)};leaf<&int32>:(p<&R>){->&(part(p).n)};owner<R>:{->nested:{->n:7};->tag:"x"};view:leaf(&owner);x:*view"#,
    );
    rejects(
        r#"<R>:<{n<int32>}>;leaf<&int32>:(p<&R>,other<&string>){->&(p.n)};owner<R>:{->n:7};view:{short:"x";->leaf(&owner,&short)}"#,
        "E303",
    );
    rejects(
        "<R>:<{n<int32>}>;leaf<&int32>:(p<&R>){->&(p.n)};owner<R>:={->n:7};view:leaf(&owner);owner={->n:8};x:*view",
        "E302",
    );
}

#[test]
pub(crate) fn list_element_contracts_visit_types_without_enumerating_capacity() {
    for source in [
        "first<&int32>:(p<&int32[2]>){->&(p[1])};a:[1,2];r:first(&a);v:*r",
        "first<&int32>:(p<&int32[65536]>){->&(p[1])};a<int32[65536]>:[1];r:first(&a);v:*r",
        "first<&int32>:(p<&int32[2][2]>){->&(p[1][1])};a:[[1,2],[3,4]];r:first(&a);v:*r",
        "first<&int32>:(p<&{items<{value<int32>}[2]>}>){->&(p.items[1].value)};a:{->items:[{->value:1},{->value:2}]};r:first(&a);v:*r",
    ] {
        accepts(source);
    }
    rejects("first<&int32>:(p<int32[2]>){->&(p[1])}", "E303");
    rejects(
        "first<&int32>:(p<&int32[2]>,other<&string>){->&(p[1])};wrap<&int32>:(p<&int32[2]>){local:\"x\";->first(p,&local)}",
        "E303",
    );
    rejects("r:{a:[1,2];->&(a[1])}", "E303");
    rejects("a:[1,2];r:a.{->&(self[1])}", "E303");
}

#[test]
pub(crate) fn projected_candidates_stop_at_the_contract_budget() {
    let fields = (0..64)
        .map(|id| format!("n{id}<int32>;"))
        .collect::<String>();
    let output = (0..64)
        .map(|id| format!("r{id}<&int32>;"))
        .collect::<String>();
    let values = (0..64)
        .map(|id| format!("->n{id}:{id};"))
        .collect::<String>();
    let returns = (0..64)
        .map(|id| format!("->r{id}:&(p.n0);"))
        .collect::<String>();
    let source = format!(
        "<R>:<{{{fields}}}>;get<{{{output}}}>:(p<&R>){{{returns}}};owner<R>:{{{values}}};view:get(&owner)"
    );
    let errors = crate::compile(&source).unwrap_err();
    assert_eq!(errors[0].code, "B001", "{errors:?}");
    assert!(errors[0].message.contains("budget"));
    let returns = (0..64)
        .map(|id| format!("->r{id}:&(p[1].n0);"))
        .collect::<String>();
    let source = format!(
        "<R>:<{{{fields}}}>;get<{{{output}}}>:(p<&R[1]>){{{returns}}};owner<R[1]>:[{{{values}}}];view:get(&owner)"
    );
    let errors = crate::compile(&source).unwrap_err();
    assert_eq!(errors[0].code, "B001", "{errors:?}");
    assert!(errors[0].message.contains("budget"));
}
#[test]
pub(crate) fn input_reference_chains_keep_symbolic_dereference_components() {
    let int = Type::Int {
        bits: 32,
        signed: true,
    };
    let pointer = Type::Reference(Box::new(int));
    let cell = Type::Reference(Box::new(pointer.clone()));
    let mut flow = Flow::new();
    let state = input(7, &cell, &mut flow, Span::default()).unwrap();
    assert_eq!(state.origins.len(), 2);
    for (path, ty) in [(Vec::new(), &cell), (vec![Step::Deref], &pointer)] {
        assert_eq!(component_type(&cell, &path), Some(ty));
        assert!(state.origins.iter().any(|origin| origin.component == path
            && origin.source
                == Source::Input {
                    id: 7,
                    component: path.clone(),
                    fields: Vec::new()
                }));
    }
    assert!(
        projections(&cell, &pointer, &mut flow, Span::default())
            .unwrap()
            .is_empty()
    );
}

#[test]
pub(crate) fn transitive_function_inputs_preserve_reference_cells_and_carrier_members() {
    for source in [
        "<P>:<&int32>;load<P>:(p<&P>){->*p};owner:7;cell:&owner;result:load(&cell);value:*result",
        "<C>:<{view<&int32>}>;copy<C>:(p<&C>){->*p};owner:7;holder<C>:{->view:&owner};result:copy(&holder);value:*(result.view)",
        "<P>:<&int32>;<C>:<{view<P>}>;cell<&P>:(p<&C>){->&(p.view)};owner:7;holder<C>:{->view:&owner};result:cell(&holder);value:*(*result)",
        "<I>:<{view<&int32>}>;<C>:<{inner<I>;number<int32>}>;part<&I>:(p<&C>){->&(p.inner)};owner:7;holder<C>:{->inner:{->view:&owner};->number:8};result:part(&holder);value:*(result.view)",
    ] {
        accepts(source);
    }
}

#[test]
pub(crate) fn call_bounds_survive_dereferencing_returned_reference_summaries() {
    accepts("owner:1;copy:{holder:{->view:&owner};pointer:&holder;->*pointer};value:*(copy.view)");
    accepts("owner:1;result:{cell:&owner;pointer:&cell;->*pointer};value:*result");
    rejects(
        "<C>:<{view<&int32>}>;copy<C>:(p<&C>){->*p};owner:1;result:{holder<C>:{->view:&owner};->copy(&holder)};value:*(result.view)",
        "E303",
    );
    rejects(
        "<P>:<&int32>;load<P>:(p<&P>){->*p};wrap<P>:(p<P>){cell:p;->load(&cell)}",
        "E303",
    );
    rejects(
        "<P>:<&int32>;id<&P>:(p<&P>,other<&string>){->p};owner:1;cell:&owner;result:{short:\"s\";->*(id(&cell,&short))};value:*result",
        "E303",
    );
    rejects(
        "<P>:<&int32>;id<&P>:(p<&P>,other<&string>){->p};owner:=1;cell:&owner;other:=\"s\";result:*(id(&cell,&other));other=\"changed\";value:*result",
        "E302",
    );
    accepts(
        "<C>:<{view<&int32>;value<int32>}>;copy<C>:(p<&C>,other<&string>){->*p};owner:=1;holder<C>:{->value:7;->view:&owner};other:=\"s\";number:copy(&holder,&other).value;other=\"changed\";owner=2",
    );
}

pub(crate) fn nullable_carrier_state(
    id: usize,
    owner: Option<usize>,
    carrier: &Type,
    field: &Type,
    flow: &mut Flow,
) -> State {
    let span = Span::default();
    let pointer = Type::Reference(Box::new(Type::Int {
        bits: 32,
        signed: true,
    }));
    let state = if let Some(owner) = owner {
        let value = State::default()
            .borrowed(
                Source::Local {
                    id: owner,
                    fields: Vec::new(),
                },
                &Type::Int {
                    bits: 32,
                    signed: true,
                },
                flow,
                span,
            )
            .unwrap();
        value
            .inject(
                field
                    .members()
                    .iter()
                    .position(|ty| ty == &pointer)
                    .unwrap(),
                flow,
                span,
            )
            .unwrap()
    } else {
        State::default()
            .inject(
                field
                    .members()
                    .iter()
                    .position(|ty| ty == &Type::Null)
                    .unwrap(),
                flow,
                span,
            )
            .unwrap()
    };
    state
        .prefix(&[Step::Slot(1)])
        .borrowed(
            Source::Local {
                id,
                fields: Vec::new(),
            },
            carrier,
            flow,
            span,
        )
        .unwrap()
}

#[test]
pub(crate) fn candidate_choices_preserve_pointee_variants_and_exclude_unrelated_origins() {
    let span = Span::default();
    let int = Type::Int {
        bits: 32,
        signed: true,
    };
    let pointer = Type::Reference(Box::new(int.clone()));
    let field = Type::union([Type::Null, pointer.clone()]);
    let carrier = Type::Record {
        primary: Box::new(Type::Null),
        fields: vec![crate::hir::Field {
            name: "view".into(),
            ty: field.clone(),
            mutable: false,
        }],
    };
    let reference = Type::Reference(Box::new(carrier.clone()));
    let mut flow = Flow::new();
    let empty = nullable_carrier_state(0, None, &carrier, &field, &mut flow);
    let present = nullable_carrier_state(1, Some(2), &carrier, &field, &mut flow);
    let extra = State::default()
        .borrowed(
            Source::Local {
                id: 3,
                fields: Vec::new(),
            },
            &int,
            &mut flow,
            span,
        )
        .unwrap();
    let result = call(
        &reference,
        &[
            (&reference, empty.clone()),
            (&reference, present),
            (&pointer, extra.clone()),
        ],
        &mut flow,
        span,
    )
    .unwrap();
    let path = [Step::Deref, Step::Slot(1)];
    let null = result.member(
        &path,
        field
            .members()
            .iter()
            .position(|ty| ty == &Type::Null)
            .unwrap(),
        &mut flow,
    );
    let borrowed = result.member(
        &path,
        field
            .members()
            .iter()
            .position(|ty| ty == &pointer)
            .unwrap(),
        &mut flow,
    );
    assert_ne!(flow.and(result.proof, null), FALSE);
    assert_ne!(flow.and(result.proof, borrowed), FALSE);
    assert_eq!(flow.and(null, borrowed), FALSE);
    for origin in &result.origins {
        if origin.component.is_empty() {
            let opposite = if origin.source
                == (Source::Local {
                    id: 0,
                    fields: Vec::new(),
                }) {
                borrowed
            } else {
                null
            };
            assert_eq!(flow.and(origin.guard, opposite), FALSE);
        } else {
            assert!(
                origin.source
                    == Source::Local {
                        id: 2,
                        fields: Vec::new()
                    }
            );
        }
    }
    let nested = leaves(&reference, &result, &mut flow, span)
        .unwrap()
        .into_iter()
        .find(|leaf| !leaf.component.is_empty())
        .unwrap();
    assert!(
        result
            .bounds
            .iter()
            .any(|bound| bound.component == nested.component
                && bound.source
                    == Source::Local {
                        id: 3,
                        fields: Vec::new()
                    })
    );
    let result = call(
        &reference,
        &[(&reference, empty), (&pointer, extra)],
        &mut flow,
        span,
    )
    .unwrap();
    assert_eq!(
        result.member(
            &path,
            field
                .members()
                .iter()
                .position(|ty| ty == &pointer)
                .unwrap(),
            &mut flow
        ),
        FALSE
    );
    assert!(
        result
            .origins
            .iter()
            .all(|origin| origin.component.is_empty())
    );
    assert_eq!(result.proof, TRUE);
    let flag = flow.fresh();
    let mut guarded =
        nullable_carrier_state(0, None, &carrier, &field, &mut flow).under(flag, &mut flow);
    let present = nullable_carrier_state(1, Some(2), &carrier, &field, &mut flow)
        .under(flow.not(flag), &mut flow);
    guarded.merge(present, &mut flow, span).unwrap();
    let result = call(&reference, &[(&reference, guarded)], &mut flow, span).unwrap();
    let null = result.member(
        &path,
        field
            .members()
            .iter()
            .position(|ty| ty == &Type::Null)
            .unwrap(),
        &mut flow,
    );
    let borrowed = result.member(
        &path,
        field
            .members()
            .iter()
            .position(|ty| ty == &pointer)
            .unwrap(),
        &mut flow,
    );
    let wrong_null = flow.and(null, flow.not(flag));
    let wrong_borrowed = flow.and(borrowed, flag);
    assert_eq!(flow.and(result.proof, wrong_null), FALSE);
    assert_eq!(flow.and(result.proof, wrong_borrowed), FALSE);
    let selected_null = flow.and(null, flag);
    let selected_borrowed = flow.and(borrowed, flow.not(flag));
    assert_ne!(flow.and(result.proof, selected_null), FALSE);
    assert_ne!(flow.and(result.proof, selected_borrowed), FALSE);
}

#[test]
pub(crate) fn nullable_pointees_do_not_gain_references_from_ignored_inputs() {
    accepts(
        "<C>:<{view<&int32><null>}>;id<&C>:(p<&C>,other<&int32>){->p};holder<C>:{};result:{short:1;->*(id(&holder,&short))};|result.view<null>|ok:true",
    );
    rejects(
        "<C>:<{view<&int32><null>}>;id<&C>:(p<&C>,other<&int32>){->p};owner:1;holder<C>:{->view:&owner};result:{short:2;->*(id(&holder,&short))}",
        "E303",
    );
}

#[test]
pub(crate) fn transitive_return_bounds_stop_at_the_shared_origin_budget() {
    let pointer = Type::Reference(Box::new(Type::Int {
        bits: 32,
        signed: true,
    }));
    let carrier = Type::Record {
        primary: Box::new(Type::Null),
        fields: (0..32)
            .map(|id| crate::hir::Field {
                name: format!("view{id}"),
                ty: pointer.clone(),
                mutable: false,
            })
            .collect(),
    };
    let reference = Type::Reference(Box::new(carrier));
    let mut flow = Flow::new();
    let states = (0..16)
        .map(|id| input(id, &reference, &mut flow, Span::default()).unwrap())
        .collect::<Vec<_>>();
    let args = states
        .into_iter()
        .map(|state| (&reference, state))
        .collect::<Vec<_>>();
    let error = match call(&reference, &args, &mut flow, Span::default()) {
        Ok(_) => panic!("unbounded summary expansion"),
        Err(error) => error,
    };
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("budget"));
}

#[test]
pub(crate) fn reference_return_candidates_preserve_modes_and_exact_pointee_types() {
    use crate::hir::Type;
    let shared = Type::Reference(Box::new(Type::Bool));
    let exclusive = Type::Exclusive(Box::new(Type::Bool));
    let other = Type::Reference(Box::new(Type::Int {
        bits: 32,
        signed: true,
    }));
    for (result, input, expected) in [
        (&shared, &shared, true),
        (&shared, &exclusive, true),
        (&exclusive, &exclusive, true),
        (&exclusive, &shared, false),
        (&shared, &other, false),
        (&shared, &Type::Bool, false),
    ] {
        assert_eq!(super::returns::candidate(result, input), expected);
    }
}
