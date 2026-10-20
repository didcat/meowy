use super::super::capture::tests::capture;
use super::*;

pub(super) fn setup() -> (Checker, Snapshots, Port, Input) {
    let (checker, _, _, mut snapshots) = capture("flag:false;|flag|1");
    let (&point, fact) = snapshots.guards.iter().next().unwrap();
    let input = fact.input;
    snapshots.guards.clear();
    (checker, snapshots, Port::Normal(point), input)
}

#[test]
pub(crate) fn scalar_guard_authority_uses_historical_reads_and_copies() {
    let (_, _, _, snapshots) = capture("flag:=false;copy:flag;|flag|1;flag=true;|copy|2;|flag|3");
    let mut facts = snapshots.guards.values().collect::<Vec<_>>();
    facts.sort_by_key(|fact| fact.input.point);
    assert_eq!(
        facts.iter().map(|fact| fact.authority).collect::<Vec<_>>(),
        [
            Authority::Boolean(1),
            Authority::Boolean(1),
            Authority::Boolean(2)
        ]
    );
    let (_, _, _, snapshots) = capture("f:(b<boolean>){|b|1};f(true)");
    assert_eq!(
        snapshots.guards.values().next().unwrap().authority,
        Authority::Boolean(3)
    );
}

#[test]
pub(crate) fn scalar_guard_authority_keeps_unknown_bottom_and_stopped_inputs_distinct() {
    for source in ["f<boolean>:(){->true};|f()|1", "|1==1|1"] {
        let (_, _, _, snapshots) = capture(source);
        assert_eq!(
            snapshots.guards.values().next().unwrap().authority,
            Authority::Unknown
        );
    }
    let (_, _, _, snapshots) = capture("f:(n<never>){|true|1}");
    assert_eq!(
        snapshots.guards.values().next().unwrap().authority,
        Authority::Bottom
    );
    let (_, _, _, snapshots) =
        capture("d:@\"debug\";stop<never>:(){d.panic(\"stop\")};stop()&&true");
    assert!(snapshots.guards.is_empty());
}

#[test]
pub(crate) fn scalar_guard_authority_preserves_captured_and_value_dependencies() {
    for case in 0..4 {
        let (mut checker, mut snapshots, port, mut input) = setup();
        match case {
            0 => input.control = Some(true),
            1 => {
                snapshots
                    .expressions
                    .get_mut(&input.source.input)
                    .unwrap()
                    .value
                    .as_mut()
                    .unwrap()
                    .control = true
            }
            2 => input.control = None,
            3 => {
                snapshots.expressions.remove(&input.source.input);
            }
            _ => unreachable!(),
        }
        snapshots
            .guard(port, &input, &mut checker.flow, Span::default())
            .unwrap();
        let expected = if case < 2 {
            Authority::Dependent
        } else {
            Authority::Unknown
        };
        assert_eq!(
            snapshots.guards.values().next().unwrap().authority,
            expected
        );
    }
}

#[test]
pub(crate) fn scalar_guard_authority_rejects_stale_histories_and_invalid_boolean_facts() {
    for case in 0..7 {
        let (mut checker, mut snapshots, port, mut input) = setup();
        match case {
            0 => input.source.owner += 1,
            1 => input.source.control = true,
            2 => input.source.normal = false,
            3 => {
                snapshots
                    .rows
                    .get_mut(&Port::Normal(input.source.input))
                    .unwrap()
                    .after = Value::Missing
            }
            4 => {
                snapshots
                    .expressions
                    .get_mut(&input.source.input)
                    .unwrap()
                    .owner += 1
            }
            5 => {
                snapshots
                    .expressions
                    .get_mut(&input.source.input)
                    .unwrap()
                    .value
                    .as_mut()
                    .unwrap()
                    .domain
                    .null = true
            }
            6 => {
                snapshots
                    .expressions
                    .get_mut(&input.source.input)
                    .unwrap()
                    .value
                    .as_mut()
                    .unwrap()
                    .domain
                    .values = Some(Values::Bool(4))
            }
            _ => unreachable!(),
        }
        let parts = snapshots.parts;
        assert!(
            snapshots
                .guard(port, &input, &mut checker.flow, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "{case}"
        );
        assert!(snapshots.guards.is_empty());
        assert_eq!(snapshots.parts, parts);
    }
}

#[test]
pub(crate) fn scalar_guard_authority_reserves_exact_work_and_payload_before_publication() {
    let (mut checker, mut snapshots, port, input) = setup();
    let start = checker.flow.work;
    snapshots
        .guard(port, &input, &mut checker.flow, Span::default())
        .unwrap();
    let cost = checker.flow.work - start;
    let expected = snapshots.guards.clone();
    for (parts, extra, pass) in [(0, 0, false), (1, 1, false), (1, 0, true)] {
        snapshots.guards.clear();
        snapshots.parts = parts;
        checker.flow.full = false;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        let result = snapshots.guard(port, &input, &mut checker.flow, Span::default());
        assert_eq!(result.is_ok(), pass);
        if pass {
            assert_eq!(snapshots.guards, expected);
            assert_eq!(snapshots.parts, 0);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        } else {
            assert!(snapshots.guards.is_empty());
            assert_eq!(snapshots.parts, parts);
        }
    }
    checker.flow.work = 0;
    snapshots.parts = 1;
    assert!(
        snapshots
            .guard(port, &input, &mut checker.flow, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(snapshots.guards, expected);
    assert_eq!(snapshots.parts, 1);
}
