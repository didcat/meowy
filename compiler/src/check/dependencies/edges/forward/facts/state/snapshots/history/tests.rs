use super::super::capture::tests::capture;
use super::super::*;

#[test]
pub(crate) fn scalar_histories_follow_straight_line_tails_and_direct_leaves() {
    for source in [
        "{}",
        "x:=1;{->x;x=2};x",
        "x:=1;'out{->x;x=2;'out.leave()};x",
        "x:=1;'out{x=2;'out.leave();x=3};x",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        for &block in checker.bodies.keys() {
            for port in [Port::BlockNormal(block), Port::BlockResult(block)] {
                assert_eq!(
                    snapshots.rows[&port].control,
                    Some(false),
                    "{source}: {port:?}"
                );
            }
        }
        for body in checker.bodies.values() {
            if let Some(parent) = body.parent {
                assert!(snapshots.expressions.contains_key(&parent));
            }
        }
    }
}

#[test]
pub(crate) fn scalar_histories_do_not_restore_unknown_tails_from_known_slots() {
    for source in [
        "d:@\"debug\";v:{->7;d.print(\"tail\")}",
        "v:'loop{->7;|false|'loop.restart()}",
        "v:{->7;1+2}",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let target = checker
            .emissions
            .values()
            .find(|op| op.targets.len() == 1)
            .unwrap()
            .targets[0]
            .block;
        let entry = snapshots.rows[&Port::BlockEntry(target)].after;
        assert_eq!(
            matches!(entry, Value::Known(_)),
            !source.contains("restart"),
            "{source}"
        );
        assert_eq!(
            snapshots.rows[&Port::BlockResult(target)].control,
            None,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn scalar_histories_keep_function_owners_separate() {
    let (checker, _, reports, snapshots) = capture("f:(x<int32>){->x};g:(){->8};1+2");
    for (&owner, &(block, _)) in &reports.entries {
        assert_eq!(
            snapshots.rows[&Port::BlockResult(block)].control,
            (owner != 0).then_some(false)
        );
        assert_eq!(checker.bodies[&block].owner, owner);
    }
}
