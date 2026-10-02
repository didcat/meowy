use super::*;

pub(super) fn checked(source: &str) -> (Checker, Reports) {
    crate::compile(source).unwrap();
    let ast = crate::parser::parse(source).unwrap();
    let mut checker = Checker::new();
    let body = checker.block(&ast, None, None).unwrap();
    let program = crate::hir::Program {
        body,
        functions: std::mem::take(&mut checker.functions)
            .into_iter()
            .flatten()
            .collect(),
        locals: std::mem::take(&mut checker.locals),
    };
    let reports = checker.entry_reports(&program, ast.span).unwrap();
    (checker, reports)
}

#[test]
pub(crate) fn block_validation_keeps_roots_nested_partial_and_required_boundaries() {
    for source in [
        "",
        "v:{->1};f<int32>:(){->2}",
        "v<{x<int32>;y<int32>}>:{->{->x:1};->y:2}",
        "<T>:{n:1+2;-><uint8[n]>};v<T>:[7]",
        "xs<int32[1]>:[(({->1}))]",
    ] {
        let (mut checker, reports) = checked(source);
        let mut count = 0;
        for (&owner, (_, walk)) in &reports.entries {
            for &port in &walk.ports {
                if checker
                    .validate_block_effect(&reports, owner, port, Span::default())
                    .unwrap()
                {
                    count += 1;
                }
            }
        }
        assert!(count >= 2, "{source}");
    }
}

#[test]
pub(crate) fn block_validation_keeps_structural_normal_visits_distinct_from_results() {
    let source = "d:@\"debug\";'out{|false|{'out.leave()};d.panic(\"stop\")}";
    let (mut checker, reports) = checked(source);
    let id = checker
        .bodies
        .iter()
        .find_map(|(&id, body)| {
            (!body.completion.normal
                && reports.entries[&0].1.ports.contains(&Port::BlockNormal(id)))
            .then_some(id)
        })
        .unwrap();
    assert!(
        checker
            .validate_block_effect(&reports, 0, Port::BlockNormal(id), Span::default())
            .unwrap()
    );
    assert!(
        checker
            .validate_block_effect(&reports, 0, Port::BlockResult(id), Span::default())
            .is_err()
    );
    assert!(!reports.entries[&0].1.ports.contains(&Port::BlockResult(id)));
}

#[test]
pub(crate) fn block_validation_leaves_dispatch_with_its_own_reports() {
    let (mut checker, reports) = checked("v:3.{->$}");
    let id = *checker.proofs.dispatches.first().unwrap();
    for port in [Port::BlockNormal(id), Port::BlockResult(id)] {
        assert!(
            !checker
                .validate_block_effect(&reports, 0, port, Span::default())
                .unwrap()
        );
    }
}

#[test]
pub(crate) fn block_validation_accepts_grouped_effectful_list_source_spans() {
    let source = "d:@\"debug\";xs<uint8[1]><uint16[1]>:[(({d.print(\"item\");->300}))]";
    let (mut checker, reports) = checked(source);
    let id = checker
        .bodies
        .iter()
        .find_map(|(&id, body)| {
            body.parent
                .filter(|parent| checker.points[*parent].span != body.span)
                .map(|_| id)
        })
        .unwrap();
    for port in [Port::BlockNormal(id), Port::BlockResult(id)] {
        assert!(
            checker
                .validate_block_effect(&reports, 0, port, Span::default())
                .unwrap()
        );
    }
}
