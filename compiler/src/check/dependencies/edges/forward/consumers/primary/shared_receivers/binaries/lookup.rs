use super::*;
use crate::check::dependencies::operations::SharedRecord;

mod boundaries;

pub(super) fn lookup(
    checker: &mut Checker,
    reports: &Reports,
    input: PointId,
    owner: usize,
    record: SharedRecord,
) -> (Result<Option<Slot>>, usize) {
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    let actual = checker.shared_copies_primary_slot(reports, input, owner, record, Span::default());
    let work = checker.flow.work - start;
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    (actual, work)
}

pub(super) fn check_inputs(checker: &mut Checker, reports: &Reports) {
    for op in checker.binaries.values().cloned().collect::<Vec<_>>() {
        for step in 0..2 {
            if let Some(fields) = op.fields[step]
                && let BinaryClass::SharedScalar(primary) = op.types.inputs[step]
            {
                let _ = lookup(
                    checker,
                    reports,
                    op.inputs[step],
                    op.owner,
                    SharedRecord { primary, fields },
                );
            }
        }
    }
}

#[test]
pub(crate) fn counted_binary_lookup_preserves_sources_scopes_and_opaque_inputs() {
    for (ty, value) in [
        ("int8", "1"),
        ("uint64", "1"),
        ("float32", "1.5"),
        ("boolean", "true"),
        ("string", "\"cat\""),
        ("null", "null"),
    ] {
        for init in ["{->p;->a:true;->z:3}", "p.{->$;->a:true;->z:3}"] {
            let frames = [
                ("local", "a:r==p;b:p!=r".to_owned()),
                ("receiver", "out:r.{a:$==p;b:p!=$}".to_owned()),
                (
                    "copy",
                    "out:r.{copy:$;alias:((copy));a:alias==p;b:p!=alias}".to_owned(),
                ),
                (
                    "nested",
                    "out:r.{copy:$;inner:copy.{next:$;a:next==p;b:p!=next}}".to_owned(),
                ),
                (
                    "guarded",
                    "flag:=true;out:r.{copy:$;|flag|inner:3.{a:copy==p;b:p!=copy}}".to_owned(),
                ),
                (
                    "stopped",
                    "d:@\"debug\";out:r.{copy:$;a:copy==p;b:copy==d.panic(\"stop\")}".to_owned(),
                ),
                ("mutable", "out:r.{copy:=$;a:copy==p;b:p!=copy}".to_owned()),
                (
                    "function",
                    format!("f<boolean>:(p<&{ty}>){{r:{init};->r.{{copy:$;a:copy==p;->p!=copy}}}}"),
                ),
                (
                    "parameter",
                    format!("f<boolean>:(r<{{-><&{ty}>;a<boolean>;z<int32>}}>,p<&{ty}>){{->r==p}}"),
                ),
                (
                    "call",
                    format!(
                        "f<{{-><&{ty}>;a<boolean>;z<int32>}}>:(p<&{ty}>){{->p;->a:true;->z:3}};x:f(p);a:x==p"
                    ),
                ),
            ];
            for (name, frame) in frames {
                let source = format!("n<{ty}>:{value};p:&n;r:{init};{frame}");
                let (mut checker, reports) = checked(&source);
                let mut count = 0;
                let mut costs = std::collections::BTreeSet::new();
                for (&id, op) in &checker.binaries.clone() {
                    for step in 0..2 {
                        let Some(fields) = op.fields[step] else {
                            continue;
                        };
                        let BinaryClass::SharedScalar(primary) = op.types.inputs[step] else {
                            panic!()
                        };
                        let (actual, work) = lookup(
                            &mut checker,
                            &reports,
                            op.inputs[step],
                            op.owner,
                            SharedRecord { primary, fields },
                        );
                        let port = Port::Projection { point: id, step };
                        let actual = actual.unwrap();
                        assert_eq!(
                            actual.is_none(),
                            matches!(name, "mutable" | "parameter" | "call"),
                            "{source}"
                        );
                        if let Some(slot) = actual {
                            assert_eq!(
                                (slot.index, checker.bodies[&slot.block].owner),
                                (0, op.owner)
                            );
                            assert_eq!(
                                reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                                Sources::Unknown
                            );
                        }
                        assert_eq!(
                            actual,
                            reports.slot_uses.get(&port).map(|(_, slot)| *slot),
                            "{source}"
                        );
                        assert!(work > 0);
                        costs.insert(work);
                        count += 1;
                    }
                }
                assert!(count > 0, "{source}");
                if ty == "int8" {
                    eprintln!("{name} {init}: work {costs:?}");
                }
            }
        }
    }
}
