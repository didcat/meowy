use super::*;
use crate::check::dependencies::edges::{Edge, MAX_EDGES, Port, Route};

pub(crate) fn check(source: &str) -> Checker {
    let mut checker = Checker::new();
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    checker
}

#[test]
pub(crate) fn group_inputs_classify_ordinary_composed_and_short_circuit_children() {
    let source = "f<int32>:(){->1};x:((f()));y:((false&&true));z:(false||true);v<{x<boolean>;y<boolean>}>:{->(({->x:true}));->y:false}";
    crate::compile(source).unwrap();
    let checker = check(source);
    assert_eq!(checker.group_inputs.len(), 7);
    for (&id, group) in &checker.group_inputs {
        assert_eq!(group.owner, checker.points[id].owner);
        assert_eq!(group.block, checker.points[id].block);
        assert_eq!(group.span, checker.points[id].span);
        assert_eq!(checker.points[group.input].parent, Some(id));
        assert!(checker.points[group.input].complete);
        assert_eq!(
            checker.region_edges[&id],
            [
                Edge::new(Port::Entry(id), Port::Entry(group.input), Route::Next),
                Edge::new(Port::Normal(group.input), Port::Normal(id), Route::Next),
            ]
        );
    }
    for kind in [PointKind::And, PointKind::Or] {
        assert!(
            checker
                .group_inputs
                .values()
                .any(|group| { checker.points[group.input].kind == kind })
        );
    }
    assert!(checker.group_inputs.values().any(|group| {
        checker
            .bodies
            .values()
            .any(|body| body.parent == Some(group.input))
    }));
}

#[test]
pub(crate) fn group_inputs_keep_same_span_checks_and_function_owners_distinct() {
    let mut checker = Checker::new();
    let stmt = crate::parser::parse("(1)").unwrap().stmts.remove(0);
    for owner in [0, 0, 1] {
        checker.owner = owner;
        checker.stmt(&stmt).unwrap();
    }
    let groups = checker.group_inputs.values().collect::<Vec<_>>();
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0].span, groups[1].span);
    assert_eq!(groups[1].span, groups[2].span);
    assert_ne!(groups[0].input, groups[1].input);
    assert_ne!(groups[1].input, groups[2].input);
    assert_eq!(groups[0].owner, 0);
    assert_eq!(groups[2].owner, 1);
    let checker = check("f<int32>:(){->((1))};v:((f()))");
    assert!(checker.group_inputs.values().any(|group| group.owner != 0));
    assert!(checker.group_inputs.values().any(|group| group.owner == 0));
}

#[test]
pub(crate) fn group_inputs_preserve_stops_without_classifying_other_regions() {
    let checker = check("d:@\"debug\";f<never>:(){((d.panic(\"stop\")))};((f()))");
    assert_eq!(checker.group_inputs.len(), 4);
    for call in checker.invocations.values() {
        assert!(!call.may_return);
        assert!(
            !call
                .edges
                .iter()
                .any(|edge| edge.to == Port::Normal(call.point))
        );
    }
    let checker = check("'out{(({ 'out.leave() }))}");
    assert_eq!(checker.group_inputs.len(), 2);
    assert_eq!(checker.scope_exits.len(), 1);
    let checker = check("x:1;p:&x;q<&int32>:p;|true|1");
    assert!(checker.group_inputs.is_empty());
    assert!(!checker.region_edges.is_empty());
    let mut checker = Checker::new();
    checker.required = true;
    let stmt = crate::parser::parse("(1)").unwrap().stmts.remove(0);
    let crate::ast::StmtKind::Expr(expr) = stmt.kind else {
        panic!()
    };
    checker.expr_point(&expr, None).unwrap();
    assert!(checker.group_inputs.is_empty());
    assert_eq!(checker.region_edges.len(), 1);
    assert!(checker.type_work.is_none());
}

#[test]
pub(crate) fn group_inputs_preserve_source_errors_without_partial_capture() {
    for (source, code) in [
        ("x<boolean>:((1))", "E207"),
        ("x<uint8>:((256))", "E216"),
        ("((missing))", "E201"),
    ] {
        let mut checker = Checker::new();
        assert_eq!(
            checker
                .block(&crate::parser::parse(source).unwrap(), None, None)
                .unwrap_err()
                .code,
            code
        );
        assert!(checker.group_inputs.is_empty());
        assert!(checker.region_edges.is_empty());
        assert!(checker.point.is_none());
    }
}

#[test]
pub(crate) fn group_inputs_validate_identity_before_registering_edges() {
    for case in 0..13 {
        let mut checker = check("x:(1)");
        let (&id, &group) = checker.group_inputs.first_key_value().unwrap();
        checker.group_inputs.clear();
        checker.region_edges.clear();
        let mut span = group.span;
        match case {
            0 => checker.points[id].kind = PointKind::Condition,
            1 => checker.points[id].owner += 1,
            2 => checker.points[id].complete = false,
            3 => span.end += 1,
            4 => checker.points[group.input].parent = None,
            5 => checker.points[group.input].owner += 1,
            6 => checker.points[group.input].block = None,
            7 => checker.points[group.input].complete = false,
            8 => checker.points[group.input].kind = PointKind::Stmt,
            9 => {
                checker.group_inputs.insert(
                    id,
                    Group {
                        owner: usize::MAX,
                        ..group
                    },
                );
            }
            10 => {
                checker.region_edges.insert(
                    id,
                    [Edge::new(Port::Entry(id), Port::Normal(id), Route::Next); 2],
                );
            }
            11 => checker.owner += 1,
            12 => checker.points[group.input].kind = PointKind::Read,
            _ => unreachable!(),
        }
        let groups = checker.group_inputs.clone();
        let edges = checker.region_edges.clone();
        let error = checker.group_region(id, group.input, span).unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("identity"));
        assert_eq!(checker.group_inputs, groups);
        assert_eq!(checker.region_edges, edges);
    }
    let mut checker = check("x:(1)");
    let (&id, &group) = checker.group_inputs.first_key_value().unwrap();
    checker.points[id].complete = false;
    checker.point = Some(id);
    checker.group_region(id, group.input, group.span).unwrap();
    assert_eq!(checker.group_inputs.len(), 1);
}

#[test]
pub(crate) fn group_inputs_share_edge_and_work_limits_atomically() {
    for room in [0, 3, 4, 6, 7] {
        let mut checker = check("x:(1)");
        let (&id, &group) = checker.group_inputs.first_key_value().unwrap();
        checker.group_inputs.clear();
        checker.region_edges.clear();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - room;
        let result = checker.group_region(id, group.input, group.span);
        assert_eq!(result.is_ok(), room == 7);
        assert_eq!(checker.group_inputs.len(), usize::from(room == 7));
        assert_eq!(checker.region_edges.len(), usize::from(room == 7));
        assert!(checker.type_work.is_none());
    }
    let mut checker = check("x:(1)");
    let (&id, &group) = checker.group_inputs.first_key_value().unwrap();
    checker.group_inputs.clear();
    checker.region_edges.clear();
    checker.output_edges = MAX_EDGES;
    let error = checker
        .group_region(id, group.input, group.span)
        .unwrap_err();
    assert!(error.message.contains("region edge budget"));
    assert!(checker.group_inputs.is_empty());
    assert!(checker.region_edges.is_empty());
}

#[test]
pub(crate) fn group_inputs_bound_capture_and_keep_replay_at_capacity() {
    let mut checker = check("x:(1)");
    let (&id, &group) = checker.group_inputs.first_key_value().unwrap();
    checker.group_inputs.clear();
    checker.region_edges.clear();
    for key in 1..MAX_GROUPS {
        checker.group_inputs.insert(usize::MAX - key, group);
    }
    checker.group_region(id, group.input, group.span).unwrap();
    assert_eq!(checker.group_inputs.len(), MAX_GROUPS);
    checker.group_region(id, group.input, group.span).unwrap();
    assert_eq!(checker.region_edges.len(), 1);
    checker.group_inputs.remove(&id);
    checker.group_inputs.insert(usize::MAX, group);
    checker.region_edges.clear();
    let error = checker
        .group_region(id, group.input, group.span)
        .unwrap_err();
    assert!(error.message.contains("group-input budget"));
    assert_eq!(checker.group_inputs.len(), MAX_GROUPS);
    assert!(checker.region_edges.is_empty());
}
