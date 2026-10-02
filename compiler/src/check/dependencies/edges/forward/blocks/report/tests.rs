use super::{super::tests::checked, *};
use crate::check::dependencies::bodies::completion::Shape;

#[test]
pub(crate) fn block_reports_keep_independent_roots_and_checked_partial_shapes() {
    let source =
        "f<never>:(){d:@\"debug\";d.panic(\"unused\")};v<{x<int32>;y<int32>}>:{->{->x:1};->y:2}";
    let (checker, reports) = checked(source);
    let root = reports.entries[&0].0;
    let unused = reports.entries[&1].0;
    assert!(reports.blocks[&root].1.normal && reports.blocks[&root].1.result);
    assert!(!reports.blocks.contains_key(&unused));
    assert!(reports.effects.values().any(|(owner, _)| *owner == 1));
    for fields in [1, 2] {
        let (&id, body) = checker
            .bodies
            .iter()
            .find(|(_, body)| body.completion.result == Shape::Record { fields })
            .unwrap();
        let (owner, observed) = reports.blocks[&id];
        assert_eq!(owner, body.owner);
        assert_eq!(observed.parent, body.parent);
        assert_eq!(observed.completion, body.completion);
        assert!(observed.normal && observed.result);
    }
}

#[test]
pub(crate) fn block_reports_preserve_independent_visits_and_duplicate_merges() {
    for port in [Port::BlockNormal(0), Port::BlockResult(0)] {
        let (mut checker, mut reports) = checked("");
        reports.entries.get_mut(&0).unwrap().1.ports = vec![port, port];
        let blocks = checker
            .block_effects_limited(&reports, Span::default(), 1)
            .unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[&0].1.normal, port == Port::BlockNormal(0));
        assert_eq!(blocks[&0].1.result, port == Port::BlockResult(0));
    }
}

#[test]
pub(crate) fn block_reports_keep_never_normal_visits_and_stop_later_results() {
    let source = "d:@\"debug\";'out{|false|{'out.leave()};d.panic(\"stop\")};v:{->2}";
    let (checker, reports) = checked(source);
    assert!(reports.blocks.values().any(|(_, observed)| {
        observed.normal && !observed.result && !observed.completion.normal
    }));
    for (&id, body) in &checker.bodies {
        if body.completion.normal {
            assert!(!reports.blocks.contains_key(&id));
        }
    }
    let (checker, reports) = checked("v:3.{->$}");
    assert!(
        checker
            .proofs
            .dispatches
            .iter()
            .all(|id| !reports.blocks.contains_key(id))
    );
}
