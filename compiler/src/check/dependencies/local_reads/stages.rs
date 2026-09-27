use super::{tests::check, *};

#[test]
pub(crate) fn local_read_stages_precede_narrowing_and_outer_expected_conversion() {
    let source = "f:(v<int32><string>){|v<int32>|x<int32><null>:v}";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (&narrow, op) = checker
        .narrowings
        .iter()
        .find(|(_, op)| op.changed)
        .unwrap();
    let read = &checker.local_reads[&op.input];
    assert_ne!(read.owner, 0);
    assert_eq!(
        read.edges,
        [
            Edge::new(
                Port::Entry(op.input),
                Port::Operation(op.input),
                Route::Next
            ),
            Edge::new(
                Port::Operation(op.input),
                Port::Normal(op.input),
                Route::Next
            ),
        ]
    );
    assert!(op.edges.contains(&Edge::new(
        Port::Normal(op.input),
        Port::Operation(narrow),
        Route::Next
    )));
    let outer = checker.points[narrow].parent.unwrap();
    assert_eq!(checker.coercions[&outer].input, narrow);
    assert_eq!(
        checker.coercions[&outer].kind,
        super::super::CoercionKind::Convert
    );
}

#[test]
pub(crate) fn local_read_stages_keep_reference_cells_separate_from_dereference_loads() {
    let source = "n:=1;p:&n;q:p;x:*q";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (&id, deref) = checker.derefs.first_key_value().unwrap();
    let narrow = &checker.narrowings[&deref.input];
    let read = &checker.local_reads[&narrow.input];
    assert_eq!(read.local, 2);
    assert_eq!(read.storage, 2);
    assert!(read.edges.iter().all(|edge| edge.to != Port::Operation(id)));
    assert_eq!(narrow.edges.last().unwrap().to, Port::Normal(deref.input));
    assert!(deref.edges.contains(&Edge::new(
        Port::Normal(deref.input),
        Port::Operation(id),
        Route::Next
    )));
}

#[test]
pub(crate) fn local_read_stages_preserve_control_never_and_ordinary_rejections() {
    let source = "flag:false;n:1;|flag|x:n;f<never>:(p<never>){->p}";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    assert!(checker.local_reads.values().any(|op| op.control));
    let (&id, stopped) = checker
        .local_reads
        .iter()
        .find(|(_, op)| !op.normal)
        .unwrap();
    assert_eq!(
        stopped.edges,
        [Edge::new(Port::Entry(id), Port::Operation(id), Route::Next)]
    );
    for (source, code) in [
        ("v:=1;p:&v;v=2;after:*p", "E302"),
        ("f:(v<int32><null>){x<int32>:v}", "E207"),
        ("f:(v<int32><null>){x:v~<int32>}", "E208"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn local_read_stages_reject_invalid_alias_storage_without_overwriting_prior_reads() {
    let mut checker = check("row:{->value:1;copy:value}");
    let (&id, read) = checker.local_reads.first_key_value().unwrap();
    let read = read.clone();
    let count = checker.local_read_edges;
    let missing = checker.locals.len();
    checker.proofs.aliases.get_mut(&read.local).unwrap().root = missing;
    assert!(
        checker
            .capture_local_read(id, read.local, read.normal, read.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(checker.local_reads[&id], read);
    assert_eq!(checker.local_read_edges, count);
}

#[test]
pub(crate) fn local_read_stages_validate_ids_storage_and_shared_budgets_atomically() {
    let mut checker = check("v:1;x:v");
    let (&id, read) = checker.local_reads.first_key_value().unwrap();
    let read = read.clone();
    let count = checker.local_read_edges;
    checker
        .capture_local_read(id, read.local, read.normal, read.span)
        .unwrap();
    assert_eq!(checker.local_read_edges, count);
    assert!(
        checker
            .capture_local_read(id, read.local, !read.normal, read.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(checker.local_reads[&id], read);
    checker.local_reads.clear();
    checker.local_read_edges = 0;
    assert!(
        checker
            .capture_local_read(id, checker.locals.len(), true, read.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.points[id].owner += 1;
    assert!(
        checker
            .capture_local_read(id, read.local, read.normal, read.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.points[id].owner -= 1;
    checker.narrowing_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .capture_local_read(id, read.local, read.normal, read.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.local_reads.is_empty());
    assert_eq!(checker.local_read_edges, 0);
}
