use super::*;
use crate::borrow_value::{Origin, Source};
use crate::flow::{MAX_PROOF_WORK, TRUE};

pub(super) fn header(count: usize) -> Header {
    Header {
        site: 3,
        function: 0,
        body: 1,
        count,
        entered: TRUE,
    }
}

pub(super) fn state() -> State {
    State {
        origins: vec![Origin {
            component: Vec::new(),
            source: Source::Local {
                id: 0,
                fields: Vec::new(),
            },
            guard: TRUE,
        }],
        ..State::default()
    }
}

#[test]
pub(crate) fn call_input_capture_precharges_exact_payload_and_work() {
    let mut flow = Guards::new();
    let (snapshot, cost) = Inputs::capture(
        header(2),
        2,
        vec![(&Type::Bool, state()), (&Type::Bool, state())],
        &mut flow,
        5,
        Span::default(),
    )
    .unwrap();
    assert_eq!(cost, 5);
    assert_eq!(snapshot.args.len(), 2);
    assert_eq!(snapshot.normal, 2);
    assert_eq!(snapshot.header.site, 3);
    assert_eq!(
        snapshot
            .args
            .iter()
            .map(|(position, _)| *position)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    let work = flow.work;
    for (limit, remaining, succeeds) in [(4, work, false), (5, work - 1, false), (5, work, true)] {
        let mut flow = Guards::new();
        flow.work = MAX_PROOF_WORK - remaining;
        assert_eq!(
            Inputs::capture(
                header(2),
                2,
                vec![(&Type::Bool, state()), (&Type::Bool, state())],
                &mut flow,
                limit,
                Span::default()
            )
            .is_ok(),
            succeeds
        );
    }
}

#[test]
pub(crate) fn call_input_capture_rejects_invalid_or_oversized_prefixes() {
    for (count, normal, size) in [
        (1, 2, 1),
        (1, 1, 2),
        (3, 1, 1),
        (3, 0, 2),
        (MAX_FACT_ORIGINS + 1, 0, 0),
    ] {
        let args = (0..size).map(|_| (&Type::Bool, state())).collect();
        let result = Inputs::capture(
            header(count),
            normal,
            args,
            &mut Guards::new(),
            usize::MAX,
            Span::default(),
        );
        assert!(result.is_err(), "{count}/{normal}/{size}");
    }
    for (count, normal, size) in [(0, 0, 0), (3, 0, 1), (3, 1, 2), (2, 2, 2)] {
        let args = (0..size).map(|_| (&Type::Bool, state())).collect();
        let (snapshot, _) = Inputs::capture(
            header(count),
            normal,
            args,
            &mut Guards::new(),
            MAX_FACT_ORIGINS,
            Span::default(),
        )
        .unwrap();
        assert_eq!(snapshot.normal, normal);
        assert_eq!(snapshot.args.len(), size);
    }
}
