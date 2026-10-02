use super::{tests::checked, *};
use crate::check::dependencies::{
    ScalarKind,
    bodies::{MAX_BODY_RESULTS, completion::Shape},
};

pub(self) fn rejected(checker: &mut Checker, reports: &Reports, id: crate::hir::BlockId) {
    let effects = reports.effects.clone();
    let counts = checker.edge_counts();
    for port in [Port::BlockNormal(id), Port::BlockResult(id)] {
        assert!(
            checker
                .validate_block_effect(reports, 0, port, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
    assert_eq!(reports.effects, effects);
    assert_eq!(checker.edge_counts(), counts);
}

#[test]
pub(crate) fn block_validation_rejects_wrong_parents_roots_and_completion_shapes() {
    for fault in 0..13 {
        let (mut checker, reports) = checked("v:{a:1;b:2;->3};f<int32>:(){->4}");
        let (&id, body) = checker
            .bodies
            .iter()
            .find(|(_, body)| body.parent.is_some())
            .unwrap();
        let parent = body.parent.unwrap();
        let span = body.span;
        match fault {
            0 => checker.block = id,
            1 => checker.bodies.get_mut(&id).unwrap().owner = 1,
            2 => checker.bodies.get_mut(&id).unwrap().parent = None,
            3 => checker.bodies.get_mut(&id).unwrap().parent = Some(usize::MAX),
            4 => checker.points[parent].complete = false,
            5 => checker.points[parent].owner = 1,
            6 => checker.points[parent].kind = PointKind::Stmt,
            7 => checker.points[parent].block = Some(id),
            8 => checker.points[parent].span.start += 1,
            9 => checker.points[parent].span.end -= 1,
            10 => checker.bodies.get_mut(&id).unwrap().span.start = span.end + 1,
            11 => checker.bodies.get_mut(&id).unwrap().completion.normal = false,
            12 => checker.bodies.get_mut(&id).unwrap().completion.result = Shape::Never,
            _ => unreachable!(),
        }
        rejected(&mut checker, &reports, id);
    }
    for fault in 0..3 {
        let (mut checker, mut reports) = checked("v:{->1}");
        let id = reports.entries[&0].0;
        match fault {
            0 => reports.entries.get_mut(&0).unwrap().0 = usize::MAX,
            1 => {
                reports.entries.remove(&0);
            }
            2 => checker.bodies.get_mut(&id).unwrap().owner = 1,
            _ => unreachable!(),
        }
        rejected(&mut checker, &reports, id);
    }
    for result in [
        Shape::Scalar(ScalarKind::Int {
            bits: 7,
            signed: true,
        }),
        Shape::Scalar(ScalarKind::Float { bits: 16 }),
        Shape::Record {
            fields: MAX_BODY_RESULTS + 1,
        },
        Shape::List {
            capacity: crate::list::MAX_CAPACITY + 1,
        },
        Shape::Union { members: 0 },
        Shape::Union { members: 1 },
        Shape::Union {
            members: MAX_BODY_RESULTS + 1,
        },
    ] {
        let (mut checker, reports) = checked("v:{->1}");
        let id = reports.entries[&0].0;
        checker.bodies.get_mut(&id).unwrap().completion.result = result;
        rejected(&mut checker, &reports, id);
    }
}

#[test]
pub(crate) fn block_validation_rejects_sequence_sites_slots_and_adjacency_corruption() {
    for fault in 0..22 {
        let (mut checker, reports) = checked("v:{a:1;b:2;->3}");
        let (&id, body) = checker
            .bodies
            .iter()
            .find(|(_, body)| body.parent.is_some())
            .unwrap();
        let span = body.span;
        let key = SequenceSource::Block(id);
        let first = checker.sequences[&key].items[0].unwrap();
        let site = checker.points[first].site.unwrap();
        match fault {
            0 => checker.sequences.get_mut(&key).unwrap().owner = 1,
            1 => checker.sequences.get_mut(&key).unwrap().items[0] = Some(usize::MAX),
            2 => checker.sequences.get_mut(&key).unwrap().items[1] = Some(first),
            3 => checker.points[first].complete = false,
            4 => checker.points[first].owner = 1,
            5 => checker.points[first].kind = PointKind::Expr,
            6 => checker.points[first].block = None,
            7 => checker.points[first].parent = None,
            8 => checker.points[first].span.start = span.start - 1,
            9 => checker.points[first].span.end = span.end + 1,
            10 => checker.points[first].site = None,
            11 => checker.points[first].site = Some(checker.statements),
            12 => checker.sites.get_mut(&site).unwrap().complete = false,
            13 => checker.sites.get_mut(&site).unwrap().owner = 1,
            14 => checker.sites.get_mut(&site).unwrap().block = None,
            15 => checker.sites.get_mut(&site).unwrap().point = None,
            16 => checker.sites.get_mut(&site).unwrap().span.end += 1,
            17 => {
                checker.sequences.get_mut(&key).unwrap().edges.pop();
            }
            18 => checker.sequences.get_mut(&key).unwrap().edges.swap(0, 1),
            19 => checker.sequences.get_mut(&key).unwrap().edges[0].route = Route::Result,
            20 => {
                let edge = checker.sequences[&key].edges[0];
                checker.sequences.get_mut(&key).unwrap().edges.push(edge);
            }
            21 => {
                let reversed = Span {
                    start: span.end,
                    end: span.start,
                };
                checker.points[first].span = reversed;
                checker.sites.get_mut(&site).unwrap().span = reversed;
            }
            _ => unreachable!(),
        }
        rejected(&mut checker, &reports, id);
    }
}

#[test]
pub(crate) fn block_validation_rejects_endpoint_order_missing_links_and_extras() {
    for parent in [false, true] {
        for fault in 0..4 {
            let (mut checker, reports) = checked("v:{a:1;b:2;->3}");
            let (&id, body) = checker
                .bodies
                .iter()
                .find(|(_, body)| body.parent.is_some())
                .unwrap();
            let key = if parent {
                SequenceSource::Expr(body.parent.unwrap())
            } else {
                SequenceSource::Block(id)
            };
            let edges = checker.endpoints.get_mut(&key).unwrap();
            match fault {
                0 => {
                    edges.pop();
                }
                1 => edges.swap(0, 1),
                2 => edges.push(edges[0]),
                3 => edges[0].route = Route::Result,
                _ => unreachable!(),
            }
            rejected(&mut checker, &reports, id);
        }
    }
}

#[test]
pub(crate) fn block_validation_preserves_unknown_sequence_barriers_without_bridging() {
    for gap in 0..3 {
        let (mut checker, reports) = checked("v:{a:1;b:2;->3}");
        let (&id, _) = checker
            .bodies
            .iter()
            .find(|(_, body)| body.parent.is_some())
            .unwrap();
        let key = SequenceSource::Block(id);
        let sequence = checker.sequences.get_mut(&key).unwrap();
        let old = sequence.edges.clone();
        sequence.items[gap] = None;
        sequence.edges = sequence
            .items
            .windows(2)
            .filter_map(|pair| {
                Some(Edge::new(
                    Port::Normal(pair[0]?),
                    Port::Entry(pair[1]?),
                    Route::Next,
                ))
            })
            .collect();
        let ends = checker.endpoints.get_mut(&key).unwrap();
        if gap == 0 {
            ends.remove(1);
        }
        if gap == 2 {
            ends.remove(2);
        }
        for port in [Port::BlockNormal(id), Port::BlockResult(id)] {
            assert!(
                checker
                    .validate_block_effect(&reports, 0, port, Span::default())
                    .unwrap()
            );
        }
        checker.sequences.get_mut(&key).unwrap().edges = old;
        rejected(&mut checker, &reports, id);
    }
}
