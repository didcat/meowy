use super::*;

#[test]
pub(crate) fn scalar_domain_containment_agrees_with_canonical_joins() {
    let mut flow = Flow::default();
    let span = Span::default();
    let values = [
        Domain::BOTTOM,
        Domain::NULL,
        Domain::boolean(false),
        Domain::boolean(true),
        Domain {
            values: Some(Values::Bool(3)),
            null: true,
        },
        Domain::integer(i128::MIN, &mut flow, span).unwrap(),
        Domain::integer(7, &mut flow, span).unwrap(),
        Domain {
            values: Some(Values::Int(Integers::Set(vec![1, 7, 100]))),
            null: false,
        },
        Domain {
            values: Some(Values::Int(Integers::Interval(0, 100))),
            null: false,
        },
        Domain {
            values: Some(Values::Int(Integers::Interval(i128::MIN, i128::MAX))),
            null: true,
        },
    ];
    for left in &values {
        for right in &values {
            let expected = left
                .join(right, &mut flow, span)
                .is_ok_and(|value| value == *right);
            let start = flow.work;
            assert_eq!(left.fits(right, &mut flow, span).unwrap(), expected);
            let cost = flow.work - start;
            for extra in [0, 1] {
                let mut limited = Flow {
                    work: crate::flow::MAX_PROOF_WORK - cost + extra,
                    ..Flow::default()
                };
                let result = left.fits(right, &mut limited, span);
                assert_eq!(result.is_ok(), extra == 0);
                if let Ok(found) = result {
                    assert_eq!(found, expected);
                }
            }
        }
    }
    for invalid in [
        Domain {
            values: Some(Values::Bool(0)),
            null: false,
        },
        Domain {
            values: Some(Values::Int(Integers::Set(vec![]))),
            null: false,
        },
        Domain {
            values: Some(Values::Int(Integers::Set(vec![2, 1]))),
            null: false,
        },
        Domain {
            values: Some(Values::Int(Integers::Interval(0, 1))),
            null: false,
        },
    ] {
        assert!(invalid.fits(&Domain::NULL, &mut flow, span).is_err());
        assert!(Domain::BOTTOM.fits(&invalid, &mut flow, span).is_err());
    }
}

#[test]
pub(crate) fn scalar_domain_joins_preserve_order_independence_null_and_bottom() {
    let mut flow = Flow::default();
    let span = Span::default();
    let integers = vec![
        Domain::BOTTOM,
        Domain::NULL,
        Domain::integer(-3, &mut flow, span).unwrap(),
        Domain {
            values: Some(Values::Int(Integers::Set((0..16).collect()))),
            null: false,
        },
        Domain {
            values: Some(Values::Int(Integers::Interval(0, 100))),
            null: true,
        },
    ];
    let booleans = vec![
        Domain::BOTTOM,
        Domain::NULL,
        Domain::boolean(false),
        Domain::boolean(true),
    ];
    for values in [integers, booleans] {
        for left in &values {
            assert_eq!(left.join(left, &mut flow, span).unwrap(), *left);
            for right in &values {
                let joined = left.join(right, &mut flow, span).unwrap();
                assert_eq!(joined, right.join(left, &mut flow, span).unwrap());
                for last in &values {
                    let tail = right.join(last, &mut flow, span).unwrap();
                    assert_eq!(
                        joined.join(last, &mut flow, span).unwrap(),
                        left.join(&tail, &mut flow, span).unwrap()
                    );
                }
            }
        }
    }
}

#[test]
pub(crate) fn scalar_domain_hulls_do_not_encode_null_as_an_integer_endpoint() {
    let mut flow = Flow::default();
    let span = Span::default();
    let mut value = Domain::NULL;
    for number in (1..=33).step_by(2) {
        let next = Domain::integer(number, &mut flow, span).unwrap();
        value = value.join(&next, &mut flow, span).unwrap();
    }
    assert_eq!(
        value,
        Domain {
            values: Some(Values::Int(Integers::Interval(1, 33))),
            null: true
        }
    );
}
