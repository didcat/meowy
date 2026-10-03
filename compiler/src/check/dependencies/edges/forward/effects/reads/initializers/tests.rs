use super::{super::super::tests::checked, *};

#[test]
pub(crate) fn read_initializer_inputs_preserve_outer_roots_owners_and_control() {
    let source = "flag:false;r<{n<int32>}>:(({->n:1}));|flag|r;outer:{copy:r};f<int32>:(){n:2;->n}";
    crate::compile(source).unwrap();
    let (mut checker, reports) = checked(source, true);
    let reads: Vec<_> = checker
        .local_reads
        .iter()
        .map(|(&id, op)| (id, op.owner, op.local))
        .collect();
    assert!(reads.iter().any(|(_, owner, _)| *owner != 0));
    assert!(checker.local_reads.values().any(|op| op.control));
    assert!(reads.iter().any(|(id, _, local)| {
        checker.points[*id].block != checker.points[reports.initializers[local].statement].block
    }));
    for (id, owner, local) in reads {
        let expected = reports.initializers[&local].input.unwrap();
        assert_ne!(checker.points[expected].parent, Some(id));
        assert_eq!(
            checker
                .read_initializer_input(&reports, id, owner, Span::default())
                .unwrap(),
            Some(expected)
        );
    }
}

#[test]
pub(crate) fn read_initializer_inputs_keep_special_cells_and_missing_evidence_unknown() {
    let source = "n:=1;copy:n;box:{->n:2;copy:n};v:3.{copy:$;->$};f<int32>:(x<int32>){->x};g<never>:(x<never>){->x}";
    crate::compile(source).unwrap();
    let (mut checker, reports) = checked(source, false);
    let reads: Vec<_> = checker
        .local_reads
        .iter()
        .map(|(&id, op)| (id, op.owner))
        .collect();
    assert!(reads.len() >= 5);
    for (id, owner) in reads {
        assert_eq!(
            checker
                .read_initializer_input(&reports, id, owner, Span::default())
                .unwrap(),
            None
        );
    }
    for missing in 0..4 {
        let (mut checker, mut reports) = checked("n:1;copy:n", false);
        let (&id, op) = checker.local_reads.first_key_value().unwrap();
        let local = op.local;
        match missing {
            0 => {
                reports.effects.remove(&id);
            }
            1 => {
                reports.initializers.remove(&local);
            }
            2 => {
                reports.eligible.remove(&local);
            }
            3 => {
                let init = reports.initializers.get_mut(&local).unwrap();
                init.input = None;
                let op = checker.operations.get_mut(&init.statement).unwrap();
                op.input = None;
                op.edges.drain(..2);
                let (_, Effect::Storage { input, .. }) =
                    reports.effects.get_mut(&init.statement).unwrap()
                else {
                    panic!()
                };
                *input = None;
            }
            _ => unreachable!(),
        }
        assert_eq!(
            checker
                .read_initializer_input(&reports, id, 0, Span::default())
                .unwrap(),
            None
        );
    }
}

#[test]
pub(crate) fn read_initializer_inputs_reject_corrupt_read_and_binding_identities_atomically() {
    for fault in 0..17 {
        let (mut checker, mut reports) = checked("n:1;copy:n", false);
        let (&id, op) = checker.local_reads.first_key_value().unwrap();
        let local = op.local;
        let statement = reports.initializers[&local].statement;
        let (
            owner,
            Effect::Read {
                local,
                storage,
                normal,
                control,
            },
        ) = reports.effects.get_mut(&id).unwrap()
        else {
            panic!()
        };
        match fault {
            0 => *owner = 9,
            1 => *local = usize::MAX,
            2 => *storage = usize::MAX,
            3 => *normal = false,
            4 => *control = !*control,
            5 => {
                checker.local_reads.remove(&id);
            }
            6 => checker.local_reads.get_mut(&id).unwrap().edges.clear(),
            7 => checker.points[id].owner = 9,
            8 => checker.points[id].complete = false,
            9 => {
                reports.index.operations.remove(&id);
            }
            10 => reports.initializers.get_mut(local).unwrap().owner = 9,
            11 => reports.initializers.get_mut(local).unwrap().statement = id,
            12 => reports.initializers.get_mut(local).unwrap().input = None,
            13 => {
                reports.effects.remove(&statement);
            }
            14 => checker
                .operations
                .get_mut(&statement)
                .unwrap()
                .edges
                .clear(),
            15 => {
                reports.entries.remove(&0);
            }
            16 => {
                let root = reports.initializers[local].input.unwrap();
                checker.points[root].parent = Some(id);
            }
            _ => unreachable!(),
        }
        let effects = reports.effects.clone();
        let index = reports.initializers.clone();
        let counts = checker.edge_counts();
        let parts = reports.parts;
        assert_eq!(
            checker
                .read_initializer_input(&reports, id, 0, Span::default())
                .unwrap_err()
                .code,
            "B001",
            "fault {fault}"
        );
        assert_eq!(reports.effects, effects);
        assert_eq!(reports.initializers, index);
        assert_eq!(checker.edge_counts(), counts);
        assert_eq!(reports.parts, parts);
    }
}
