use super::*;

#[test]
pub(crate) fn scalar_integer_constructors_bound_actual_singleton_and_interval_work() {
    for (low, high) in [
        (1, 0),
        (i128::MIN, i128::MIN),
        (i128::MAX, i128::MAX),
        (-8, 7),
        (-8, 8),
        (i128::MIN, i128::MAX),
    ] {
        let mut flow = Flow::default();
        let span = Span::default();
        let expected = Integers::interval(low, high, &mut flow, span).unwrap();
        let cost = flow.work;
        assert!(cost <= LIMIT + 1);
        for extra in [0, 1] {
            flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
            flow.full = false;
            let result = Integers::interval(low, high, &mut flow, span);
            assert_eq!(result.is_ok(), extra == 0);
            if let Ok(value) = result {
                assert_eq!(value, expected);
            }
        }
    }
    for values in [vec![], vec![i128::MIN], vec![i128::MAX]] {
        let mut flow = Flow::default();
        let span = Span::default();
        let expected = Integers::set(&values, &mut flow, span).unwrap();
        let cost = flow.work;
        assert!(cost <= 2);
        for extra in [0, 1] {
            flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
            flow.full = false;
            let result = Integers::set(&values, &mut flow, span);
            assert_eq!(result.is_ok(), extra == 0);
            if let Ok(value) = result {
                assert_eq!(value, expected);
            }
        }
    }
}

#[test]
pub(crate) fn scalar_integer_domains_canonicalize_sets_intervals_and_empty_values() {
    let mut flow = Flow::default();
    let span = Span::default();
    assert_eq!(Integers::set(&[], &mut flow, span).unwrap(), None);
    assert_eq!(Integers::interval(1, 0, &mut flow, span).unwrap(), None);
    assert_eq!(
        Integers::set(&[7, 1, 7, 3], &mut flow, span).unwrap(),
        Some(Integers::Set(vec![1, 3, 7]))
    );
    assert_eq!(
        Integers::interval(-8, 7, &mut flow, span).unwrap(),
        Some(Integers::Set((-8..=7).collect()))
    );
    assert_eq!(
        Integers::interval(-8, 8, &mut flow, span).unwrap(),
        Some(Integers::Interval(-8, 8))
    );
    let values = (0..=16).map(|n| n * 2).collect::<Vec<_>>();
    assert_eq!(
        Integers::set(&values, &mut flow, span).unwrap(),
        Some(Integers::Interval(0, 32))
    );
    assert_eq!(
        Integers::set(&[9; 100], &mut flow, span).unwrap(),
        Some(Integers::Set(vec![9]))
    );
}

#[test]
pub(crate) fn scalar_integer_domains_preserve_extremes_without_span_overflow() {
    let mut flow = Flow::default();
    let span = Span::default();
    for value in [i128::MIN, i64::MIN as i128, u64::MAX as i128, i128::MAX] {
        assert_eq!(
            Integers::interval(value, value, &mut flow, span).unwrap(),
            Some(Integers::Set(vec![value]))
        );
    }
    assert_eq!(
        Integers::interval(i128::MIN, i128::MAX, &mut flow, span).unwrap(),
        Some(Integers::Interval(i128::MIN, i128::MAX))
    );
    let values = [i128::MAX, i128::MIN];
    assert_eq!(
        Integers::set(&values, &mut flow, span).unwrap(),
        Some(Integers::Set(vec![i128::MIN, i128::MAX]))
    );
    assert!(Integers::Interval(i128::MIN, i128::MAX).valid());
}

#[test]
pub(crate) fn scalar_integer_domain_joins_keep_finite_gaps_and_use_canonical_hulls() {
    let mut flow = Flow::default();
    let span = Span::default();
    let first = Integers::Set((0..8).collect());
    let second = Integers::Set((8..16).collect());
    let full = first.join(&second, &mut flow, span).unwrap();
    assert_eq!(full, Integers::Set((0..16).collect()));
    assert_eq!(
        full.join(&Integers::Set(vec![20]), &mut flow, span)
            .unwrap(),
        Integers::Interval(0, 20)
    );
    let sparse = Integers::Set(vec![-100, 100]);
    assert_eq!(
        sparse
            .join(&Integers::Set(vec![0]), &mut flow, span)
            .unwrap(),
        Integers::Set(vec![-100, 0, 100])
    );
    let interval = Integers::Interval(0, 100);
    assert_eq!(
        interval.join(&sparse, &mut flow, span).unwrap(),
        Integers::Interval(-100, 100)
    );
    for value in [first, second, full, sparse, interval] {
        assert_eq!(value.join(&value, &mut flow, span).unwrap(), value);
    }
}

#[test]
pub(crate) fn scalar_integer_domains_reject_noncanonical_payloads_and_bound_exact_work() {
    let mut flow = Flow::default();
    let span = Span::default();
    let valid = Integers::Set(vec![0]);
    for invalid in [
        Integers::Set(vec![]),
        Integers::Set(vec![1, 1]),
        Integers::Set(vec![2, 1]),
        Integers::Set((0..17).collect()),
        Integers::Interval(0, 15),
        Integers::Interval(1, 0),
    ] {
        assert!(!invalid.valid());
        assert!(
            valid
                .join(&invalid, &mut flow, span)
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
    let values = (0..17).collect::<Vec<_>>();
    let start = flow.work;
    let expected = Integers::set(&values, &mut flow, span).unwrap();
    let cost = flow.work - start;
    for extra in [0, 1] {
        flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        flow.full = false;
        let result = Integers::set(&values, &mut flow, span);
        assert_eq!(result.is_ok(), extra == 0);
        if let Ok(value) = result {
            assert_eq!(value, expected);
        }
    }
}
