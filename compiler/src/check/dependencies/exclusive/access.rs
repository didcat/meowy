use super::{tests::check, *};

#[test]
pub(crate) fn exclusive_access_keeps_known_unknown_and_stopped_index_decisions() {
    for (source, lengths, normal) in [
        ("xs:[{->n:=1}];p:&!(xs[1].n)", vec![Some(1)], vec![true]),
        ("xs:=[1];p:&!(xs[1])", vec![None], vec![true]),
        ("r:{->xs:=[1]};p:&!(r.xs[1])", vec![None], vec![true]),
        (
            "xs:=[[1]];'out{p:&!(xs[{'out.leave()}][1])}",
            vec![None, None],
            vec![false, true],
        ),
        (
            "xs:=[[1]];'out{p:&!(xs[1][{'out.leave()}])}",
            vec![None, None],
            vec![true, false],
        ),
    ] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        let op = checker.exclusives.values().next().unwrap();
        assert_eq!(
            op.access
                .iter()
                .map(|input| input.length)
                .collect::<Vec<_>>(),
            lengths
        );
        assert_eq!(
            op.access
                .iter()
                .map(|input| input.normal)
                .collect::<Vec<_>>(),
            normal
        );
        assert_eq!(op.normal, normal.iter().all(|value| *value));
    }
    let (mut checker, _) = check("xs:[{->n:=1}];p:&!(xs[1].n)");
    let (&id, op) = checker.exclusives.first_key_value().unwrap();
    let local = op.place.root;
    checker.lengths.get_mut(&local).unwrap().length = 0;
    assert_eq!(checker.exclusives[&id].access[0].length, Some(1));
}

#[test]
pub(crate) fn exclusive_access_rejects_misaligned_and_conflicting_plans_atomically() {
    let (mut checker, body) = check("xs:=[[1]];p:&!(xs[1][1])");
    let hir::Stmt::Bind { value, .. } = &body.stmts[1] else {
        panic!()
    };
    let (&id, op) = checker.exclusives.first_key_value().unwrap();
    let plan = Plan {
        steps: op.steps.clone(),
        access: op.access.clone(),
    };
    checker.exclusives.clear();
    checker.exclusive_edges = 0;
    for fault in 0..4 {
        let mut plan = plan.clone();
        match fault {
            0 => plan.access.clear(),
            1 => plan.access.push(plan.access[0]),
            2 => plan.access[0].normal = false,
            3 => plan.access[0].length = Some(2),
            _ => unreachable!(),
        }
        assert!(
            checker
                .exclusive_operation(id, value, plan)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert!(checker.exclusives.is_empty());
        assert_eq!(checker.exclusive_edges, 0);
    }
    checker
        .exclusive_operation(id, value, plan.clone())
        .unwrap();
    checker.exclusives.get_mut(&id).unwrap().access[0].length = Some(1);
    let before = checker.exclusives.clone();
    let edges = checker.exclusive_edges;
    assert!(checker.exclusive_operation(id, value, plan).is_err());
    assert_eq!(checker.exclusives, before);
    assert_eq!(checker.exclusive_edges, edges);
}
