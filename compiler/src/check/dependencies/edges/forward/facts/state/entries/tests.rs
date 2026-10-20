use super::super::super::versions::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_entry_states_seed_declared_parameters_without_future_bind_or_write_values() {
    let (mut checker, program, reports) = context(
        "a:7;copy:a;x:=1;x=2;f<int32>:(n<int32>,other<int32>){local:n;v:=n;v=2;->local};f(7,7)",
    );
    let (versions, events, entries, _) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(entries.len(), 2);
    for (&owner, (port, state)) in &entries {
        assert_eq!(*port, Port::BlockEntry(reports.entries[&owner].0));
        assert_eq!(state.owner, owner);
        assert_eq!(state.reach, TRUE);
    }
    for (&local, slot) in &versions {
        let cell = entries[&slot.version.owner].1.get(local).unwrap();
        match slot.version.definition {
            Definition::Parameter(position) => {
                assert_eq!(
                    cell.data,
                    Data::Initialized {
                        version: Some(slot.version),
                        domain: slot.declared.clone(),
                        origin: Some(Origin::Parameter(slot.version.owner - 1, position))
                    }
                );
            }
            Definition::Bind(_) => assert_eq!(cell.data, Data::Uninitialized),
            Definition::Write(_) => panic!(),
        }
    }
    for event in events.values() {
        let version = event.version.unwrap();
        assert_eq!(
            entries[&version.owner].1.get(version.local).unwrap().data,
            Data::Uninitialized
        );
    }
    assert!(versions[&0].value.is_none());
    let (candidates, parts) = checker
        .scalar_copy_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(candidates[&0].value.is_some());
    let (candidate_entries, _) = checker
        .scalar_entry_states(&reports, &candidates, &events, Span::default(), parts)
        .unwrap();
    assert_eq!(entries, candidate_entries);
    assert_eq!(entries[&0].1.get(0).unwrap().data, Data::Uninitialized);
}

#[test]
pub(crate) fn scalar_entry_states_keep_unused_reordered_and_uninhabited_frames_distinct() {
    let (mut checker, mut program, reports) = context(
        "d:@\"debug\";impossible<never>:(n<never>){->n};stop<never>:(){d.panic(\"stop\")};unused:(flag<boolean><null>,view<&int32>){}",
    );
    program.functions.reverse();
    let (_, _, entries, _) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(entries.len(), 4);
    for function in &program.functions {
        let (port, state) = &entries[&(function.id + 1)];
        assert_eq!(*port, Port::BlockEntry(function.body.id));
        if function.name == "impossible" {
            assert_eq!(state.reach, FALSE);
            assert_eq!(state.weight(), 1);
        } else {
            assert_eq!(state.reach, TRUE);
        }
        if function.name == "unused" {
            let Data::Initialized {
                domain: Some(domain),
                ..
            } = &state.get(function.params[0]).unwrap().data
            else {
                panic!()
            };
            assert!(domain.null && domain.values.is_some());
            assert!(matches!(
                state.get(function.params[1]).unwrap().data,
                Data::Initialized {
                    domain: None,
                    origin: None,
                    version: Some(_)
                }
            ));
        }
    }
}

#[test]
pub(crate) fn scalar_entry_states_leave_alias_and_opaque_mutable_storage_unavailable() {
    let (mut checker, program, reports) = context(
        "flag:=false;row:'out{|flag|{'out->n:=1;n=2};|!flag|{'out->n:=null;n=null}};text:=\"cat\";text=\"dog\"",
    );
    let (_, events, entries, _) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let state = &entries[&0].1;
    assert_eq!(state.reach, TRUE);
    for event in events.values() {
        let (_, Effect::Storage { local, .. }) = event.source else {
            panic!()
        };
        if event.version.is_none() {
            assert!(state.get(*local).is_none());
        }
    }
}

#[test]
pub(crate) fn scalar_entry_states_merge_nested_bind_ids_without_using_point_order() {
    let (mut checker, program, reports) =
        context("outer<int32>:={inner:=1;->inner};outer=2;stable:7");
    let (_, events, entries, _) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let binds = events
        .values()
        .filter_map(|event| event.version)
        .filter(|version| matches!(version.definition, Definition::Bind(_)))
        .collect::<Vec<_>>();
    assert!(binds.windows(2).any(|pair| pair[0].local > pair[1].local));
    for version in binds {
        assert_eq!(
            entries[&version.owner].1.get(version.local).unwrap().data,
            Data::Uninitialized
        );
    }
}
