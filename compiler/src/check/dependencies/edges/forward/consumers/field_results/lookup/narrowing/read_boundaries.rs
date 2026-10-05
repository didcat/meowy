use super::*;
use crate::check::dependencies::edges::forward::{
    consumers::tests::checked, results::inputs::graph::Visit,
};

#[test]
pub(crate) fn local_field_sources_require_read_and_initializer_evidence() {
    for missing in 0..8 {
        let (mut checker, mut reports) =
            checked("r:{->n:1};n:r.n;s:{->((n~<int32>))};box:{->tag:1};temp:*(&7)");
        let (&key, &(_, direct)) = reports
            .direct_sources
            .iter()
            .find(|(_, (_, direct))| direct.source.is_some())
            .unwrap();
        let (&id, op) = checker
            .local_reads
            .iter()
            .find(|(id, _)| checker.points[**id].block == checker.points[direct.point].block)
            .unwrap();
        let local = op.local;
        let statement = reports.initializers[&local].statement;
        match missing {
            0 => {
                reports.effects.remove(&id);
            }
            1 => {
                reports.eligible.remove(&local);
            }
            2 => {
                reports.initializers.remove(&local);
            }
            3 => {
                let op = checker.local_reads.get_mut(&id).unwrap();
                op.normal = false;
                op.edges.pop();
                let (_, Effect::Read { normal, .. }) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                *normal = false;
            }
            4 => {
                reports.initializers.get_mut(&local).unwrap().input = None;
                let op = checker.operations.get_mut(&statement).unwrap();
                op.input = None;
                op.edges.drain(..2);
                let (_, Effect::Storage { input, .. }) =
                    reports.effects.get_mut(&statement).unwrap()
                else {
                    panic!()
                };
                *input = None;
            }
            5 => {
                let site = *checker.proofs.temporaries.values().next().unwrap();
                checker.proofs.temporaries.insert(local, site);
            }
            6 => {
                checker.proofs.receivers.insert(local);
            }
            7 => {
                let mut alias = checker.proofs.aliases.values().next().unwrap().clone();
                alias.root = local;
                checker.proofs.aliases.insert(local, alias);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker
                .field_narrowing_source(
                    &mut Lookup::new(&reports, MAX_EDGES),
                    direct.point,
                    0,
                    Span::default(),
                    MAX_GROUPS
                )
                .unwrap(),
            None
        );
        let sources = checker.direct_sources(&reports, Span::default()).unwrap().0;
        assert_eq!(sources[&key].1.point, direct.point);
        assert_eq!(sources[&key].1.source, None);
        assert!(
            checker
                .direct_walk_report(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        reports.direct_sources = sources;
        let walk = checker
            .direct_walk_report(&reports, Span::default())
            .unwrap()
            .0;
        assert!(
            walk.visits
                .contains(&Visit::Value(key, reports.candidate_inputs[&key].1))
        );
        assert!(
            !walk
                .visits
                .iter()
                .any(|visit| matches!(visit, Visit::Field(_, _, _)))
        );
    }
}

#[test]
pub(crate) fn local_field_sources_keep_special_cells_calls_and_loads_opaque() {
    for source in [
        "r:{->n:1};n:=r.n;s:{->((n))}",
        "box:{->n:1;inner:{->((n))}}",
        "r:{->n:1};s:r.{inner:{->($.n)}}",
        "f<int32>:(n<int32>){copy:n;->((copy))}",
        "f<never>:(n<never>){->n}",
        "r:{->n:1};n:r.n;p:&n;s:{->(*p)}",
        "r:{->n:1};s:{->*(&(r.n))}",
        "f<int32>:(){->1};n:f();s:{->((n))}",
        "r<{n<int32><null>}>:{->n:1};n:r.n;|n<int32>|s:{->((n))}",
    ] {
        let (_, reports) = checked(source);
        assert!(
            reports
                .direct_sources
                .values()
                .all(|(_, direct)| direct.source.is_none()),
            "{source}"
        );
        assert!(
            !reports
                .expanded_walk
                .visits
                .iter()
                .any(|visit| matches!(visit, Visit::Field(_, _, _))),
            "{source}"
        );
    }
}

#[test]
pub(crate) fn local_field_sources_preserve_seeded_read_and_binding_control_independently() {
    for read_control in [false, true] {
        for bind_control in [false, true] {
            let (mut checker, mut reports) = checked("r:{->n:1};n:r.n;s:{->n}");
            let direct = reports
                .direct_sources
                .values()
                .find(|(_, direct)| direct.source.is_some())
                .unwrap()
                .1;
            let (&id, op) = checker
                .local_reads
                .iter()
                .find(|(id, _)| checker.points[**id].block == checker.points[direct.point].block)
                .unwrap();
            let statement = reports.initializers[&op.local].statement;
            checker.local_reads.get_mut(&id).unwrap().control = read_control;
            checker.operations.get_mut(&statement).unwrap().control = bind_control;
            let (_, Effect::Read { control, .. }) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            *control = read_control;
            let (_, Effect::Storage { control, .. }) = reports.effects.get_mut(&statement).unwrap()
            else {
                panic!()
            };
            *control = bind_control;
            let before = format!("{reports:?}{:?}", checker.edge_counts());
            assert_eq!(
                checker.direct_sources(&reports, Span::default()).unwrap().0,
                reports.direct_sources
            );
            assert_eq!(
                checker
                    .direct_walk_report(&reports, Span::default())
                    .unwrap()
                    .0,
                reports.expanded_walk
            );
            assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        }
    }
}
