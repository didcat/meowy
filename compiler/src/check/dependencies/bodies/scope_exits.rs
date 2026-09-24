use super::super::edges::Port;
use super::{tests::check, *};

#[test]
pub(crate) fn exit_sources_link_exact_nested_targets_and_original_call_spans() {
    let source = "flag:=false;'out{'inner{|flag|'out.leave();|flag|'inner.restart()}}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let mut count = 0;
    for (block, body) in &checker.bodies {
        for ((fact, span), point) in body.facts.iter().zip(&body.sources) {
            if !matches!(fact, Fact::Leave(_) | Fact::Restart(_)) {
                continue;
            }
            let point = point.unwrap();
            let exit = &checker.scope_exits[&point];
            assert_eq!(*span, exit.span);
            assert_eq!(checker.points[point].block, Some(*block));
            assert_eq!(exit.owner, body.owner);
            match (fact, exit.edge.to) {
                (Fact::Leave(target), Port::Leave(found)) => {
                    assert_eq!(*target, found);
                    assert_ne!(*block, found);
                    assert_eq!(&source[span.start..span.end], "'out.leave()");
                }
                (
                    Fact::Restart(site),
                    Port::Restart {
                        site: found,
                        target,
                    },
                ) => {
                    assert_eq!(*site, found);
                    assert_eq!(checker.restart_inputs[site].point, Some(point));
                    assert_eq!(*block, target);
                }
                _ => panic!(),
            }
            count += 1;
        }
    }
    assert_eq!(count, 2);
}

#[test]
pub(crate) fn exit_sources_preserve_derived_control_and_function_isolation() {
    let mut checker = Checker::new();
    checker.derived.insert(0);
    let source =
        "flag:false;'out{|flag|'out.leave();'loop{|flag|'loop.restart()}};f:(){'end{'end.leave()}}";
    let block = crate::parser::parse(source).unwrap();
    checker.block(&block, None, None).unwrap();
    assert_eq!(checker.scope_exits.len(), 3);
    for (point, exit) in &checker.scope_exits {
        assert_eq!(exit.control, exit.owner == 0);
        let block = checker.points[*point].block.unwrap();
        assert!(checker.bodies[&block].sources.contains(&Some(*point)));
    }
}

#[test]
pub(crate) fn exit_sources_keep_synthetic_leave_and_restart_provenance_unknown() {
    let mut checker = Checker::new();
    let body = hir::Block {
        id: 7,
        ty: hir::Type::Null,
        stmts: vec![
            hir::Stmt::Leave {
                target: 7,
                point: None,
            },
            hir::Stmt::Restart {
                target: 7,
                site: 42,
            },
        ],
    };
    checker.track_body(&body, Span::default()).unwrap();
    assert_eq!(checker.bodies[&7].sources, [None, None]);
    assert!(checker.scope_exits.is_empty());
}

#[test]
pub(crate) fn exit_sources_reject_malformed_targets_and_missing_records_atomically() {
    for case in 0..4 {
        let (mut checker, root) = check("'out{'out.leave()}");
        let hir::Stmt::Expr(expr) = &root.stmts[0] else {
            panic!()
        };
        let hir::ExprKind::Block(scope) = &expr.kind else {
            panic!()
        };
        let mut scope = scope.clone();
        let hir::Stmt::Leave { target, point } = &mut scope.stmts[0] else {
            panic!()
        };
        let id = point.unwrap();
        match case {
            0 => *target = usize::MAX,
            1 => {
                checker.scope_exits.remove(&id);
            }
            2 => checker.scope_exits.get_mut(&id).unwrap().owner += 1,
            3 => checker.points[id].complete = false,
            _ => unreachable!(),
        }
        let count = checker.body_facts;
        let error = checker.track_body(&scope, Span::default()).unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("identity mismatch"));
        assert_eq!(checker.body_facts, count);
        assert_eq!(checker.bodies[&scope.id].sources, [Some(id)]);
    }
}
