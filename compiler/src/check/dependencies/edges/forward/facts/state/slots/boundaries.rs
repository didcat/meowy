use super::tests::frame;
use super::*;

#[test]
pub(crate) fn scalar_conditional_slot_joins_reserve_exact_work_and_tree_payload_atomically() {
    for wide in [false, true] {
        for domain in [
            Domain::NULL,
            Domain::boolean(true),
            Domain::integer(7, &mut Flow::new(), Span::default()).unwrap(),
        ] {
            let mut flow = Flow::new();
            let known = Cell {
                data: Data::Initialized {
                    version: None,
                    domain: Some(domain.clone()),
                    origin: Some(Origin::Expression(0, 1)),
                },
                control: false,
            };
            let empty = Cell {
                data: Data::Uninitialized,
                control: true,
            };
            let left = frame(Some(&known), &mut flow);
            let mut right = frame(Some(&empty), &mut flow);
            if wide {
                for block in [1, 3] {
                    right
                        .put_slot(block, &known, &mut flow, Span::default(), MAX_EDGES)
                        .unwrap();
                }
            }
            let saved = format!("{left:?}{right:?}");
            let start = flow.work;
            let expected = left
                .join(&right, &mut flow, Span::default(), MAX_EDGES)
                .unwrap();
            let cost = flow.work - start;
            let used = expected.allocation();
            assert_eq!(
                expected.slot(0).unwrap(),
                &Cell {
                    data: Data::Conditional {
                        domain,
                        origin: Some(Origin::Expression(0, 1))
                    },
                    control: true
                }
            );
            if wide {
                for block in [1, 3] {
                    assert_eq!(expected.slot(block).unwrap().data, Data::Unavailable);
                }
            }
            for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
                flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
                flow.full = false;
                let result = left.join(&right, &mut flow, Span::default(), room);
                assert_eq!(result.is_ok(), pass);
                if let Ok(actual) = result {
                    assert_eq!(actual, expected);
                    assert_eq!(actual.allocation(), used);
                }
                assert_eq!(format!("{left:?}{right:?}"), saved);
            }
        }
    }
}
