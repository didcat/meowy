use super::*;

pub(super) fn checked(source: &str) -> Checker {
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    checker
}

pub(super) fn control(checker: &mut Checker, id: PointId) -> Result<Option<bool>> {
    dependency(
        id,
        checker.branch_inputs[&id],
        &checker.points,
        &checker.branch_inputs,
        &mut checker.flow,
        Span::default(),
    )
}

#[test]
pub(crate) fn scalar_guard_controls_follow_short_circuit_availability_across_nested_arms() {
    let mut checker =
        checked("flag:false;flag&&(true||false);plain:true&&false;f:(b<boolean>){b&&true}");
    let inner = checker
        .points
        .iter()
        .position(|point| point.kind == PointKind::Or)
        .unwrap();
    assert!(checker.branch_inputs[&inner].control);
    checker.branch_inputs.get_mut(&inner).unwrap().control = false;
    assert_eq!(control(&mut checker, inner).unwrap(), Some(true));
    let function = *checker
        .branch_inputs
        .iter()
        .find(|(_, source)| source.owner == 1)
        .unwrap()
        .0;
    assert_eq!(control(&mut checker, function).unwrap(), Some(false));
    let plain = *checker
        .branch_inputs
        .keys()
        .find(|id| {
            checker.points[**id].kind == PointKind::And
                && **id > inner
                && checker.points[**id].owner == 0
        })
        .unwrap();
    assert_eq!(control(&mut checker, plain).unwrap(), Some(false));
}

#[test]
pub(crate) fn scalar_guard_controls_do_not_inherit_a_later_decision_into_its_condition() {
    let mut checker = checked("flag:false;(true||flag)&&true");
    let inner = checker
        .points
        .iter()
        .position(|point| point.kind == PointKind::Or)
        .unwrap();
    let outer = checker
        .points
        .iter()
        .position(|point| point.kind == PointKind::And)
        .unwrap();
    assert_eq!(control(&mut checker, inner).unwrap(), Some(false));
    assert_eq!(control(&mut checker, outer).unwrap(), Some(true));
}

#[test]
pub(crate) fn scalar_guard_controls_preserve_unknown_parents_and_known_dependencies() {
    let mut checker = checked("flag:false;true&&(false||true)");
    let inner = checker
        .points
        .iter()
        .position(|point| point.kind == PointKind::Or)
        .unwrap();
    let outer = checker
        .points
        .iter()
        .position(|point| point.kind == PointKind::And)
        .unwrap();
    checker.branch_inputs.remove(&outer);
    assert_eq!(control(&mut checker, inner).unwrap(), None);
    checker.branch_inputs.get_mut(&inner).unwrap().control = true;
    assert_eq!(control(&mut checker, inner).unwrap(), Some(true));
}

#[test]
pub(crate) fn scalar_guard_controls_reject_cyclic_foreign_and_incomplete_ancestry() {
    for fault in 0..3 {
        let mut checker = checked("flag:false;true&&(false||true)");
        let inner = checker
            .points
            .iter()
            .position(|point| point.kind == PointKind::Or)
            .unwrap();
        let parent = checker.points[inner].parent.unwrap();
        match fault {
            0 => checker.points[parent].parent = Some(parent),
            1 => checker.points[parent].owner += 1,
            2 => checker.points[parent].complete = false,
            _ => unreachable!(),
        }
        assert!(
            control(&mut checker, inner)
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
}

#[test]
pub(crate) fn scalar_guard_controls_charge_exact_ancestry_work() {
    let mut checker = checked("flag:false;true&&(false||true)");
    let inner = checker
        .points
        .iter()
        .position(|point| point.kind == PointKind::Or)
        .unwrap();
    let start = checker.flow.work;
    assert_eq!(control(&mut checker, inner).unwrap(), Some(false));
    let cost = checker.flow.work - start;
    checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + 1;
    assert!(
        control(&mut checker, inner)
            .unwrap_err()
            .message
            .contains("budget")
    );
    checker.flow.full = false;
    checker.flow.work = crate::flow::MAX_PROOF_WORK - cost;
    assert_eq!(control(&mut checker, inner).unwrap(), Some(false));
    assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
}
