use super::*;

#[test]
pub(crate) fn scalar_cell_comparisons_preserve_identity_and_bound_observed_payload_work() {
    let version = Version {
        owner: 0,
        local: 7,
        definition: versions::Definition::Bind(1),
    };
    let values = [
        Data::Unavailable,
        Data::Uninitialized,
        Data::Partial,
        Data::Conditional {
            domain: Domain::NULL,
            origin: None,
        },
        Data::Conditional {
            domain: Domain::boolean(false),
            origin: Some(Origin::Expression(0, 2)),
        },
        Data::Initialized {
            version: None,
            domain: None,
            origin: None,
        },
        Data::Initialized {
            version: Some(version),
            domain: Some(Domain::boolean(false)),
            origin: Some(Origin::Expression(0, 2)),
        },
        Data::Initialized {
            version: Some(Version {
                definition: versions::Definition::Write(3),
                ..version
            }),
            domain: Some(Domain::boolean(false)),
            origin: Some(Origin::Expression(0, 2)),
        },
    ];
    let cells = values
        .into_iter()
        .flat_map(|data| {
            [
                Cell {
                    data: data.clone(),
                    control: false,
                },
                Cell {
                    data,
                    control: true,
                },
            ]
        })
        .collect::<Vec<_>>();
    for left in &cells {
        for right in &cells {
            let mut flow = Flow::default();
            let span = Span::default();
            assert_eq!(left.same(right, &mut flow, span).unwrap(), left == right);
            let cost = flow.work;
            for extra in [0, 1] {
                flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
                flow.full = false;
                let result = left.same(right, &mut flow, span);
                assert_eq!(result.is_ok(), extra == 0);
                if let Ok(value) = result {
                    assert_eq!(value, left == right);
                }
            }
        }
    }
}
