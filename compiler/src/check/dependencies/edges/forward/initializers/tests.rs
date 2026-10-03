use super::*;

pub(super) fn checked(source: &str) -> (Checker, hir::Program, Reports) {
    crate::compile(source).unwrap();
    let ast = crate::parser::parse(source).unwrap();
    let mut checker = Checker::new();
    let body = checker.block(&ast, None, None).unwrap();
    let program = hir::Program {
        body,
        functions: std::mem::take(&mut checker.functions)
            .into_iter()
            .flatten()
            .collect(),
        locals: std::mem::take(&mut checker.locals),
    };
    let reports = checker.entry_reports(&program, ast.span).unwrap();
    (checker, program, reports)
}

#[test]
pub(crate) fn initializer_index_retains_exact_roots_and_independent_owners() {
    let source = "r<{n<int32>}>:(({->n:1}));copy:r.n;f<int32>:(flag<boolean>){|flag|x:2;n:3;->n}";
    let (mut checker, mut program, reports) = checked(source);
    assert!(reports.initializers.values().any(|op| op.owner != 0));
    assert!(reports.initializers.values().any(|op| {
        checker.sites[&checker.points[op.statement].site.unwrap()].point != Some(op.statement)
    }));
    for (local, value) in &reports.initializers {
        let op = &checker.operations[&value.statement];
        assert_eq!(op.local, *local);
        assert_eq!(op.owner, value.owner);
        assert_eq!(op.input, value.input);
    }
    assert!(reports.initializers.values().any(|op| {
        op.input
            .is_some_and(|input| checker.coercions.contains_key(&input))
    }));
    program.functions.reverse();
    assert_eq!(
        checker
            .binding_initializers(&program, &reports, Span::default())
            .unwrap(),
        reports.initializers
    );
}

#[test]
pub(crate) fn initializer_index_excludes_special_cells_and_stopped_declarations() {
    let source = "d:@\"debug\";n:=1;p:&n;box:{->n:2};v:3.{copy:$;->$};temp:*(&{->4});f<int32>:(x<int32>){y:x;->y};stop:d.panic(\"stop\");later:5";
    let (checker, program, reports) = checked(source);
    assert!(!checker.proofs.aliases.is_empty());
    assert!(!checker.proofs.receivers.is_empty());
    assert!(!checker.proofs.temporaries.is_empty());
    for local in reports.initializers.keys() {
        assert!(reports.eligible.contains(local));
        assert!(!checker.proofs.aliases.contains_key(local));
        assert!(!checker.proofs.receivers.contains(local));
        assert!(!checker.proofs.temporaries.contains_key(local));
        assert!(
            !program
                .functions
                .iter()
                .any(|function| function.params.contains(local))
        );
    }
    for (&id, op) in &checker.operations {
        if !reports.effects.contains_key(&id) {
            assert!(!reports.initializers.contains_key(&op.local));
        }
    }
}
