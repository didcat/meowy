use super::*;

pub(crate) fn check(source: &str) -> (Checker, hir::Block) {
    let mut checker = Checker::new();
    let block = checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    (checker, block)
}

#[test]
pub(crate) fn output_stages_interleave_operands_and_streamed_parts_before_completion() {
    for method in ["print", "panic"] {
        let source = format!(r#"d:@"debug";show:d.{method};("a{{1}}b{{2}}").(show)"#);
        crate::compile(&source).unwrap();
        let (checker, _) = check(&source);
        let (&id, output) = checker.outputs.first_key_value().unwrap();
        assert_eq!(output.panic, method == "panic");
        assert!(output.stopped.is_none());
        assert_eq!(output.parts.len(), 4);
        let first = output.parts[1].unwrap();
        let second = output.parts[3].unwrap();
        let port = |part| Port::Output { point: id, part };
        for edge in [
            Edge::new(port(0), Port::Entry(first), Route::Returned),
            Edge::new(Port::Normal(first), port(1), Route::Next),
            Edge::new(port(1), port(2), Route::Returned),
            Edge::new(port(2), Port::Entry(second), Route::Returned),
            Edge::new(Port::Normal(second), port(3), Route::Next),
            Edge::new(port(3), Port::Operation(id), Route::Returned),
        ] {
            assert!(output.edges.contains(&edge));
        }
        if output.panic {
            assert_eq!(
                output.edges[0],
                Edge::new(Port::Entry(id), Port::Prefix(id), Route::Next)
            );
            assert_eq!(
                output.edges[1],
                Edge::new(Port::Prefix(id), port(0), Route::Returned)
            );
        }
        assert_eq!(
            output.edges.iter().any(|edge| edge.to == Port::Normal(id)),
            !output.panic
        );
    }
    for source in [r#"d:@"debug";d.print("")"#, r#"d:@"debug";d.panic("")"#] {
        let (checker, _) = check(source);
        assert_eq!(checker.outputs.len(), 1);
    }
}

#[test]
pub(crate) fn output_stages_preserve_prefixes_stopped_operands_and_checked_suffixes() {
    for method in ["print", "panic"] {
        let source = format!(
            r#"d:@"debug";stop<never>:(){{d.panic("stop")}};d.{method}("a{{stop()}}b{{1}}")"#
        );
        crate::compile(&source).unwrap();
        let (checker, _) = check(&source);
        let (&id, output) = checker
            .outputs
            .iter()
            .find(|(_, output)| output.owner == 0)
            .unwrap();
        assert_eq!(output.stopped, Some(1));
        assert!(checker.points[output.parts[3].unwrap()].complete);
        assert_eq!(
            output.edges.last().unwrap().to,
            Port::Entry(output.parts[1].unwrap())
        );
        assert!(
            !output
                .edges
                .iter()
                .any(|edge| edge.to == Port::Operation(id) || edge.to == Port::Normal(id))
        );
        assert!(checker.outputs.values().any(|output| output.owner != 0));
    }
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(
            &crate::parser::parse(r#"flag:false;d:@"debug";|flag|d.print(1)"#).unwrap(),
            None,
            None,
        )
        .unwrap();
    assert!(checker.outputs.values().next().unwrap().control);
    for (source, code) in [
        (r#"d:@"debug";d.print()"#, "E212"),
        (r#"d:@"debug";d.panic("{missing}")"#, "E201"),
    ] {
        let mut checker = Checker::new();
        assert_eq!(
            checker
                .block(&crate::parser::parse(source).unwrap(), None, None)
                .unwrap_err()
                .code,
            code
        );
        assert!(checker.outputs.is_empty());
    }
}

#[test]
pub(crate) fn output_stages_validate_parts_and_publish_with_shared_budgets() {
    let (mut checker, block) = check(r#"d:@"debug";d.print("{1}{2}")"#);
    let hir::Stmt::Expr(value) = &block.stmts[0] else {
        panic!()
    };
    let hir::ExprKind::Print { parts, .. } = &value.kind else {
        panic!()
    };
    let (&id, output) = checker.outputs.first_key_value().unwrap();
    let points = output.parts.clone();
    let count = checker.output_edges;
    checker
        .output_operation(id, false, parts, points.clone(), value.span)
        .unwrap();
    assert_eq!(checker.output_edges, count);
    checker.outputs.clear();
    checker.output_edges = 0;
    for other in [vec![points[0]; 2], vec![None; 2], vec![]] {
        assert!(
            checker
                .output_operation(id, false, parts, other, value.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert!(checker.outputs.is_empty());
    }
    checker.points[points[0].unwrap()].owner += 1;
    assert!(
        checker
            .output_operation(id, false, parts, points.clone(), value.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.points[points[0].unwrap()].owner -= 1;
    checker.exclusive_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .output_operation(id, false, parts, points, value.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.outputs.is_empty());
    assert_eq!(checker.output_edges, 0);
}
