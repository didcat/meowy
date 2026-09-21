use crate::check::Checker;

pub(crate) fn seeded(source: &str) -> Checker {
    let mut checker = Checker::new();
    let block = crate::parser::parse(source).unwrap();
    for stmt in &block.stmts {
        checker.stmt(stmt).unwrap();
    }
    for input in checker.inputs.values_mut() {
        input.derived = true;
    }
    for input in checker.bool_inputs.values_mut() {
        input.derived = true;
    }
    for input in checker.record_inputs.values_mut() {
        input.input.derived = true;
    }
    checker
}

#[test]
pub(crate) fn required_types_reject_transitive_proof_evidence() {
    for (source, tail, read) in [
        ("n:3", "<T>:{copy:n;-><uint8[copy]>}", "n"),
        ("n:3", "copy:n+1;<T>:{-><uint8[copy]>}", "copy"),
        (
            "flag:true",
            "<T>:{|flag|-><uint8>;|!flag|-><uint16>}",
            "flag",
        ),
        ("row:{->n:3;->flag:true}", "<T>:{-><uint8[row.n]>}", "row.n"),
        (
            "row:{->n:3;->flag:true}",
            "<T>:{copy:row;-><uint8[copy.n]>}",
            "row",
        ),
        (
            "row:{->n:3;->flag:true}",
            "<T>:{flag:row.flag;-><uint8>}",
            "row.flag",
        ),
        ("n:3", "p:@\"proof\";r:p.can_copy<({-><uint8[n]>})>()", "n"),
    ] {
        let mut checker = seeded(source);
        let block = crate::parser::parse(tail).unwrap();
        let error = checker.block(&block, None, None).unwrap_err();
        assert_eq!(error.code, "E225", "{tail}: {error:?}");
        assert_eq!(&tail[error.span.start..error.span.end], read);
        assert!(checker.type_work.is_none());
        assert!(checker.queries.is_empty());
    }
}

#[test]
pub(crate) fn proof_evidence_does_not_taint_fixed_type_queries_or_fresh_roots() {
    let mut checker = seeded("n:3;flag:true;row:{->n:3}");
    let source = "<N>:n<>;<B>:flag<>;<R>:row<>;<T>:{-><uint8[4]>}";
    let block = crate::parser::parse(source).unwrap();
    checker.block(&block, None, None).unwrap();
    assert!(checker.type_work.is_none());
}

#[test]
pub(crate) fn required_proof_guard_preserves_original_failure_and_budget_order() {
    use super::{MAX_WORK, Work};
    use crate::ast::Span;
    use crate::check::inputs::Input;

    let origin = Span::new(1, 2);
    let read = Span::new(4, 5);
    let mut input = Input {
        derived: true,
        work: 1,
        error: Some(Checker::error(
            "E107",
            "original initializer failure",
            origin,
        )),
        value: Some(3),
    };
    let error = Work::default().input(&input, read).unwrap_err();
    assert_eq!(error.code, "E107");
    assert_eq!(error.span, origin);
    input.error = None;
    let error = Work::default().input(&input, read).unwrap_err();
    assert_eq!(error.code, "E225");
    assert_eq!(error.span, read);
    input.work = MAX_WORK + 1;
    let error = Work::default().input(&input, read).unwrap_err();
    assert_eq!(error.code, "B001");
}
