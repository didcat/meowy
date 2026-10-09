use super::{tests::check, *};

#[test]
pub(crate) fn field_shared_primary_capture_keeps_exact_referents_and_owners() {
    for (ty, value, kind) in [
        (
            "int8",
            "1",
            ScalarKind::Int {
                bits: 8,
                signed: true,
            },
        ),
        (
            "uint64",
            "1",
            ScalarKind::Int {
                bits: 64,
                signed: false,
            },
        ),
        ("float32", "1.5", ScalarKind::Float { bits: 32 }),
        ("boolean", "true", ScalarKind::Bool),
        ("string", "\"cat\"", ScalarKind::String),
        ("null", "null", ScalarKind::Null),
    ] {
        let source = format!(
            "n<{ty}>:{value};p:&n;x:{{->p;->tag:true}}.{{->$.tag}};f<boolean>:(p<&{ty}>){{->p.{{->$;->tag:true}}.{{->$.tag}}}}"
        );
        crate::compile(&source).unwrap();
        let (checker, _) = check(&source);
        assert_eq!(checker.fields.len(), 2);
        assert!(checker.fields.values().any(|op| op.owner != 0));
        for op in checker.fields.values() {
            assert_eq!(op.shared_primary, Some(kind));
            assert_eq!((op.index, op.count), (0, 1));
            assert!(!op.load && op.normal);
        }
    }
}

#[test]
pub(crate) fn field_shared_primary_capture_keeps_implicit_loads_and_other_referents_separate() {
    for (source, load, primary) in [
        ("n:1;p:&n;r:{->p;->tag:true};q:&r;x:q.tag", true, None),
        (
            "n:1;p:&n;r:{->p;->tag:true};q:&r;x:(*q).tag",
            false,
            Some(ScalarKind::Int {
                bits: 32,
                signed: true,
            }),
        ),
        ("n:[1];p:&n;r:{->p;->tag:true};x:r.tag", false, None),
        (
            "n<int32><null>:1;p:&n;r:{->p;->tag:true};x:r.tag",
            false,
            None,
        ),
        ("n:1;p:&n;q:&p;r:{->q;->tag:true};x:r.tag", false, None),
        (
            "n:@\"memory\".heap;p:&n;r:{->p;->tag:true};x:r.tag",
            false,
            None,
        ),
        ("r:{->1;->tag:true};x:r.tag", false, None),
    ] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        assert_eq!(checker.fields.len(), 1);
        let op = checker.fields.values().next().unwrap();
        assert_eq!((op.load, op.shared_primary), (load, primary), "{source}");
    }
}

#[test]
pub(crate) fn field_shared_primary_replay_rejects_stale_referents_with_unchanged_work() {
    let mut costs = Vec::new();
    for source in [
        "n:1;r:{->n;->tag:true};v:r.tag",
        "n:1;p:&n;r:{->p;->tag:true};v:r.tag",
    ] {
        let (mut checker, body) = check(source);
        let hir::Stmt::Bind { value, .. } = body.stmts.last().unwrap() else {
            panic!()
        };
        let (&id, op) = checker.fields.first_key_value().unwrap();
        let op = op.clone();
        let edges = checker.edge_counts();
        let start = checker.flow.work;
        checker
            .field_operation(id, op.input, op.load, value, op.span)
            .unwrap();
        let work = checker.flow.work - start;
        costs.push(work);
        for short in [0, 1] {
            checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
            checker.flow.full = false;
            let result = checker.field_operation(id, op.input, op.load, value, op.span);
            assert_eq!(result.is_ok(), short == 0);
        }
        checker.flow.work = 0;
        checker.flow.full = false;
        let mut changed = value.clone();
        let hir::ExprKind::Field {
            value: receiver, ..
        } = &mut changed.kind
        else {
            panic!()
        };
        let hir::Type::Record { primary, .. } = &mut receiver.ty else {
            panic!()
        };
        **primary = hir::Type::Reference(Box::new(hir::Type::Bool));
        assert!(
            checker
                .field_operation(id, op.input, op.load, &changed, op.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(checker.fields[&id], op);
        assert_eq!(checker.edge_counts(), edges);
    }
    assert_eq!(costs[0], costs[1]);
}
