use super::*;

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
