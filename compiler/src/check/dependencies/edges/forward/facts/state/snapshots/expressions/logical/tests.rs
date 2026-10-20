use super::super::super::capture::tests::capture;
use super::*;

#[test]
pub(crate) fn scalar_logical_results_follow_boolean_tables_with_fresh_identity() {
    for or in [false, true] {
        for left in [false, true] {
            for right in [false, true] {
                let op = if or { "||" } else { "&&" };
                let (checker, _, _, snapshots) = capture(&format!("a:{left};b:{right};a{op}b"));
                let (&point, input) = checker.branch_inputs.iter().next().unwrap();
                let fact = &snapshots.expressions[&point];
                let value = fact.value.as_ref().unwrap();
                assert_eq!(
                    value.domain,
                    Domain::boolean(if or { left || right } else { left && right })
                );
                assert_eq!(value.local, None);
                assert_eq!(value.version, None);
                assert_eq!(value.origin, Some(Origin::Expression(input.owner, point)));
                assert_ne!(
                    value.origin,
                    snapshots.expressions[&input.input]
                        .value
                        .as_ref()
                        .unwrap()
                        .origin
                );
                assert!(!value.control);
            }
        }
    }
}

#[test]
pub(crate) fn scalar_logical_results_keep_parameter_uncertainty_and_edge_correlations() {
    for (expr, mask) in [
        ("a&&b", 3),
        ("a||b", 3),
        ("a&&false", 1),
        ("a||true", 2),
        ("a&&true", 3),
        ("a||false", 3),
        ("a&&a", 3),
        ("a||a", 3),
    ] {
        let (checker, _, _, snapshots) = capture(&format!("f:(a<boolean>,b<boolean>){{{expr}}}"));
        let point = *checker.branch_inputs.keys().next().unwrap();
        let value = snapshots.expressions[&point].value.as_ref().unwrap();
        assert_eq!(
            value.domain,
            Domain {
                values: Some(domain::Values::Bool(mask)),
                null: false
            },
            "{expr}"
        );
        assert_eq!(value.local, None);
        assert_eq!(value.version, None);
    }
}

#[test]
pub(crate) fn scalar_logical_results_drive_nested_guards_and_independent_matchers() {
    for source in [
        "x:=0;|((false||true)&&true)|x=1;after:x",
        "x:=0;flag<boolean>:(false||true);|flag|x=1;after:x",
        "x:=0;|true&&true|x=2;|false||true|x=1;after:x",
    ] {
        let (mut checker, _, _, snapshots) = capture(source);
        let read = *checker.local_reads.keys().next_back().unwrap();
        let value = snapshots.expressions[&read].value.as_ref().unwrap();
        assert_eq!(
            value.domain,
            Domain::integer(1, &mut checker.flow, Span::default()).unwrap()
        );
        for (&point, input) in &checker.branch_inputs {
            assert_eq!(
                snapshots.expressions.contains_key(&point),
                checker.points[point].kind != PointKind::Match
            );
            assert_eq!(snapshots.expressions[&input.input].owner, input.owner);
        }
    }
}

#[test]
pub(crate) fn scalar_logical_results_preserve_old_copies_and_original_read_history() {
    let (checker, _, _, snapshots) = capture("a:=false;b:=true;old:a||b;a=true;b=false;old;a&&b");
    let mut points = checker.branch_inputs.keys().copied().collect::<Vec<_>>();
    points.sort_unstable();
    let old = snapshots.expressions[&points[0]].value.as_ref().unwrap();
    let new = snapshots.expressions[&points[1]].value.as_ref().unwrap();
    assert_eq!(old.domain, Domain::boolean(true));
    assert_eq!(new.domain, Domain::boolean(false));
    assert_ne!(old.origin, new.origin);
    let copy = checker
        .local_reads
        .iter()
        .find(|(_, read)| read.local == 2)
        .unwrap()
        .0;
    let value = snapshots.expressions[copy].value.as_ref().unwrap();
    assert_eq!(value.domain, old.domain);
    assert_eq!(value.origin, old.origin);
    assert_eq!(value.local, Some(2));
    let first = checker.branch_inputs[&points[0]].input;
    assert_eq!(
        snapshots.expressions[&first].value.as_ref().unwrap().domain,
        Domain::boolean(false)
    );
}
