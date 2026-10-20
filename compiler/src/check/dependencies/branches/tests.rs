use super::*;

pub(super) fn check(source: &str) -> Checker {
    let mut checker = Checker::new();
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    checker
}

#[test]
pub(crate) fn branch_inputs_retain_matcher_and_short_circuit_sources_and_owners() {
    let checker = check("flag:=false;|flag|1;v:flag&&true;f:(b<boolean>){b||false}");
    assert_eq!(checker.branch_inputs.len(), 3);
    for (&id, branch) in &checker.branch_inputs {
        let point = &checker.points[id];
        let source = &checker.points[branch.input];
        assert_eq!(point.owner, branch.owner);
        assert_eq!(source.owner, point.owner);
        assert_eq!(source.block, point.block);
        let condition = source.parent.unwrap();
        assert_eq!(checker.points[condition].kind, PointKind::Condition);
        assert_eq!(checker.points[condition].parent, Some(id));
        assert!(branch.normal);
        assert!(!branch.control);
    }
    assert!(
        checker
            .branch_inputs
            .values()
            .any(|branch| branch.owner == 1)
    );
}

#[test]
pub(crate) fn branch_inputs_freeze_data_and_control_marks_before_checking_bodies() {
    for source in [
        "flag:false;|flag|1",
        "flag:false;flag&&true",
        "flag:false;flag||true",
        "flag:false;|flag|{true&&false;|true|1}",
    ] {
        let mut checker = Checker::new();
        checker.derived.insert(0);
        checker
            .block(&crate::parser::parse(source).unwrap(), None, None)
            .unwrap();
        assert!(checker.branch_inputs.values().all(|branch| branch.control));
        checker.derived.clear();
        checker.control = false;
        assert!(checker.branch_inputs.values().all(|branch| branch.control));
    }
    let mut checker = check("flag:false;|flag|1");
    checker.derived.insert(0);
    assert!(!checker.branch_inputs.values().next().unwrap().control);
}

#[test]
pub(crate) fn branch_inputs_keep_stopped_conditions_and_source_errors_distinct() {
    let checker = check("d:@\"debug\";stop<never>:(){d.panic(\"stop\")};stop()&&true");
    assert!(!checker.branch_inputs.values().next().unwrap().normal);
    for (source, code) in [
        ("|1|2", "E215"),
        ("false&&1", "E222"),
        ("|true|missing", "E201"),
    ] {
        let mut checker = Checker::new();
        assert_eq!(
            checker
                .block(&crate::parser::parse(source).unwrap(), None, None)
                .unwrap_err()
                .code,
            code
        );
        assert!(checker.branch_inputs.is_empty());
    }
}

#[test]
pub(crate) fn branch_inputs_reject_stale_roots_cycles_owners_and_conflicting_marks() {
    for case in 0..9 {
        let mut checker = check("|true|1");
        let (&id, &branch) = checker.branch_inputs.iter().next().unwrap();
        checker.branch_inputs.clear();
        let mut input = branch.input;
        match case {
            0 => input = id,
            1 => checker.points[input].parent = Some(input),
            2 => checker.points[input].owner += 1,
            3 => checker.points[input].complete = false,
            4 => checker.points[id].kind = PointKind::Expr,
            5 => checker.points[id].complete = false,
            6 => checker.points[input].block = None,
            7 => checker.owner += 1,
            8 => {
                checker.branch_inputs.insert(
                    id,
                    Branch {
                        control: true,
                        ..branch
                    },
                );
            }
            _ => unreachable!(),
        }
        let before = checker.branch_inputs.clone();
        assert!(
            checker
                .branch_input(id, input, branch.normal, branch.control, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(checker.branch_inputs, before);
    }
}

#[test]
pub(crate) fn branch_inputs_charge_exact_work_before_atomic_publication() {
    let mut checker = check("|true|1");
    let (&id, &branch) = checker.branch_inputs.iter().next().unwrap();
    checker.branch_inputs.clear();
    let start = checker.flow.work;
    checker
        .branch_input(
            id,
            branch.input,
            branch.normal,
            branch.control,
            Span::default(),
        )
        .unwrap();
    let work = checker.flow.work - start;
    checker.branch_inputs.clear();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work + 1;
    assert!(
        checker
            .branch_input(
                id,
                branch.input,
                branch.normal,
                branch.control,
                Span::default()
            )
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.branch_inputs.is_empty());
    checker.flow.full = false;
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work;
    checker
        .branch_input(
            id,
            branch.input,
            branch.normal,
            branch.control,
            Span::default(),
        )
        .unwrap();
    assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
    assert_eq!(checker.branch_inputs[&id], branch);
}

#[test]
pub(crate) fn branch_inputs_bound_new_rows_allow_republication_and_skip_required_work() {
    let mut checker = check("|true|1");
    let (&id, &branch) = checker.branch_inputs.iter().next().unwrap();
    checker.branch_inputs = (0..MAX_EDGES).map(|key| (key, branch)).collect();
    checker
        .branch_input(id, branch.input, true, false, Span::default())
        .unwrap();
    checker.branch_inputs.remove(&id);
    checker.branch_inputs.insert(MAX_EDGES, branch);
    let error = checker
        .branch_input(id, branch.input, true, false, Span::default())
        .unwrap_err();
    assert!(error.message.contains("budget"));
    assert!(!checker.branch_inputs.contains_key(&id));
    checker.branch_inputs.remove(&MAX_EDGES);
    checker
        .branch_input(id, branch.input, true, false, Span::default())
        .unwrap();
    assert_eq!(checker.branch_inputs.len(), MAX_EDGES);
    checker.required = true;
    let start = checker.flow.work;
    checker
        .branch_input(usize::MAX, usize::MAX, false, true, Span::default())
        .unwrap();
    assert_eq!(checker.flow.work, start);
    assert_eq!(checker.branch_inputs.len(), MAX_EDGES);
}
