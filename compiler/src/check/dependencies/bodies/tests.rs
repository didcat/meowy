use super::*;

pub(crate) fn check(source: &str) -> (Checker, hir::Block) {
    let mut checker = Checker::new();
    let block = crate::parser::parse(source).unwrap();
    let body = checker.block(&block, None, None).unwrap();
    (checker, body)
}

#[test]
pub(crate) fn restart_bodies_retain_reads_and_writes_on_both_sides_of_backedges() {
    let source = "flag:=false;x:=0;'loop{before:x;x=1;|flag|'loop.restart();after:x};outside:x";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let restart = &checker.restart_inputs[&0];
    let body = &checker.bodies[&restart.target];
    assert_eq!(body.owner, restart.owner);
    let facts = body.facts.iter().map(|(fact, _)| fact).collect::<Vec<_>>();
    assert_eq!(
        facts,
        vec![
            &Fact::Bind(2),
            &Fact::Read(1),
            &Fact::Write(1),
            &Fact::Branch,
            &Fact::Read(0),
            &Fact::Restart(0),
            &Fact::Bind(3),
            &Fact::Read(1),
        ]
    );
    assert_eq!(&source[body.facts[1].1.start..body.facts[1].1.end], "x");
    assert!(checker.derived.is_empty());
}

#[test]
pub(crate) fn restart_bodies_keep_nested_targets_and_function_owners_separate() {
    let source = "flag:=false;'outer{f<int32>:(v<int32>){copy:v;->copy};'inner{a:1;|flag|'outer.restart();|flag|'inner.restart()}}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let outer = checker.restart_inputs[&0].target;
    let inner = checker.restart_inputs[&1].target;
    assert!(
        checker.bodies[&outer]
            .facts
            .iter()
            .any(|(fact, _)| *fact == Fact::Block(inner))
    );
    assert!(
        !checker.bodies[&outer]
            .facts
            .iter()
            .any(|(fact, _)| matches!(fact, Fact::Bind(_)))
    );
    assert!(
        checker.bodies[&inner]
            .facts
            .iter()
            .any(|(fact, _)| *fact == Fact::Restart(0))
    );
    let function = checker.functions[0].as_ref().unwrap();
    assert_ne!(
        checker.bodies[&function.body.id].owner,
        checker.bodies[&outer].owner
    );
    assert_eq!(
        checker.body_facts,
        checker.bodies.values().map(|body| body.facts.len()).sum()
    );
}

#[test]
pub(crate) fn body_facts_cover_indices_stores_calls_and_statement_temporaries() {
    let source = "f<int32>:(v<&int32>){->*v};i:1;xs<int32[2]>:=[1,2];xs[i]=3;x:=1;r:&!x;*r=xs[i];y:f(&(i+1))";
    crate::compile(source).unwrap();
    let (checker, root) = check(source);
    let facts = &checker.bodies[&root.id].facts;
    assert!(facts.iter().any(|(fact, _)| matches!(fact, Fact::Write(_))));
    assert!(facts.iter().any(|(fact, _)| *fact == Fact::Store));
    assert!(facts.iter().any(|(fact, _)| *fact == Fact::Call(0)));
    for id in checker.proofs.temporaries.keys() {
        assert!(facts.iter().any(|(fact, _)| *fact == Fact::Bind(*id)));
    }
    assert!(
        facts
            .iter()
            .filter(|(fact, span)| matches!(fact, Fact::Read(_))
                && &source[span.start..span.end] == "i")
            .count()
            >= 3
    );
}

#[test]
pub(crate) fn invalid_bodies_preserve_errors_and_do_not_publish_summaries() {
    for (source, code) in [("x:1;x=2", "E305"), ("x<boolean>:1", "E207")] {
        let mut checker = Checker::new();
        let block = crate::parser::parse(source).unwrap();
        assert_eq!(checker.block(&block, None, None).unwrap_err().code, code);
        assert!(checker.bodies.is_empty());
    }
    let errors = crate::compile("x:=1;r:&x;x=2;y:*r").unwrap_err();
    assert_eq!(errors[0].code, "E302");
}

#[test]
pub(crate) fn body_fact_bounds_and_identity_fail_without_partial_publication() {
    let (mut checker, body) = check("x:1;y:x");
    let count = checker.body_facts;
    checker.track_body(&body, Span::default()).unwrap();
    assert_eq!(checker.body_facts, count);
    checker.owner += 1;
    assert_eq!(
        checker.track_body(&body, Span::default()).unwrap_err().code,
        "B001"
    );
    assert_eq!(checker.body_facts, count);
    for capacity in [true, false] {
        let mut checker = Checker::new();
        if capacity {
            checker.body_facts = MAX_BODY_FACTS;
        } else {
            assert!(!checker.flow.spend(usize::MAX));
        }
        assert_eq!(
            checker.track_body(&body, Span::default()).unwrap_err().code,
            "B001"
        );
        assert!(checker.bodies.is_empty());
    }
}
