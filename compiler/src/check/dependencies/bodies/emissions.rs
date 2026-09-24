use super::{tests::check, *};

#[test]
pub(crate) fn emitted_facts_link_direct_and_fanout_sources_without_span_matching() {
    let source = "row:'out{source:{->a:1;->b:true};{'out->source}}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let mut count = 0;
    for (block, body) in &checker.bodies {
        for ((fact, _), point) in body.facts.iter().zip(&body.sources) {
            let Fact::Emit(id) = fact else { continue };
            let (found, index) = checker.emission_sources[id];
            assert_eq!(*point, Some(found));
            let emission = &checker.emissions[&found];
            assert_eq!(emission.targets[index].id, *id);
            assert_eq!(emission.owner, body.owner);
            assert_eq!(checker.points[found].block, Some(*block));
            count += 1;
        }
    }
    assert_eq!(count, 5);
}

#[test]
pub(crate) fn emitted_facts_keep_function_owners_and_unknown_synthetic_sources() {
    let (checker, _) = check("f:(){->1};->2");
    assert_eq!(checker.emissions.len(), 2);
    for (point, emission) in &checker.emissions {
        let block = checker.points[*point].block.unwrap();
        assert_eq!(checker.bodies[&block].owner, emission.owner);
        assert!(checker.bodies[&block].sources.contains(&Some(*point)));
    }
    let mut checker = Checker::new();
    let span = Span::default();
    let body = hir::Block {
        id: 7,
        ty: hir::Type::Null,
        stmts: vec![hir::Stmt::Emit {
            id: 42,
            target: 7,
            field: None,
            value: hir::Expr {
                kind: hir::ExprKind::Null,
                ty: hir::Type::Null,
                span,
            },
        }],
    };
    checker.track_body(&body, span).unwrap();
    assert_eq!(checker.bodies[&7].sources, [None]);
}

#[test]
pub(crate) fn emitted_facts_reject_malformed_targets_indices_and_owners_atomically() {
    for case in 0..5 {
        let (mut checker, mut body) = check("->1");
        let hir::Stmt::Emit {
            id, target, field, ..
        } = &mut body.stmts[0]
        else {
            panic!()
        };
        let (point, _) = checker.emission_sources[id];
        match case {
            0 => *target = usize::MAX,
            1 => *field = Some("wrong".into()),
            2 => checker.emission_sources.get_mut(id).unwrap().1 = usize::MAX,
            3 => {
                checker.emissions.remove(&point);
            }
            4 => checker.emissions.get_mut(&point).unwrap().owner += 1,
            _ => unreachable!(),
        }
        let count = checker.body_facts;
        let error = checker.track_body(&body, Span::default()).unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("identity mismatch"));
        assert_eq!(checker.body_facts, count);
        assert_eq!(checker.bodies[&body.id].sources, [Some(point)]);
    }
}
