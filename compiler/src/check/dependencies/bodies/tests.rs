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

#[test]
pub(crate) fn restart_bodies_retain_erased_scalar_and_record_inputs() {
    let source = "n:3;flag:true;row:{->n:2;->flag:false};again:=false;'loop{<N>:{-><uint8[n]>};<B>:{b:flag;-><uint8>};<F>:{b:row.flag;-><uint8[row.n]>};<R>:{copy:row;->copy<>};|again|'loop.restart()}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let target = checker.restart_inputs[&0].target;
    let inputs = &checker.body_inputs[&target];
    let reads = inputs
        .iter()
        .map(|input| &source[input.span.start..input.span.end])
        .collect::<Vec<_>>();
    assert_eq!(reads, ["n", "flag", "row.flag", "row.n", "row"]);
    assert!(inputs.iter().all(|input| !input.control));
    assert_eq!(inputs[0].id, 0);
    assert_eq!(inputs[1].id, 1);
    assert_eq!(inputs[2].id, inputs[4].id);
    assert_eq!(
        checker.body_facts,
        checker
            .bodies
            .values()
            .map(|body| body.facts.len())
            .sum::<usize>()
            + inputs.len()
    );
}

#[test]
pub(crate) fn body_inputs_exclude_skipped_reads_and_fixed_type_signatures() {
    let source = "p:@\"proof\";n:3;flag:true;again:=false;'loop{q:p.can_copy<uint8>();<N>:n<>;<F>:q.always<>;<T>:{|false|x:n;-><uint8>};<B>:{b:false&&flag;-><uint8>};<R>:{revision:p.revision;-><uint8>};|again|'loop.restart()}";
    let (checker, _) = check(source);
    assert!(checker.body_inputs.is_empty());
    assert!(!checker.queries[0].control);
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "B001");
}

#[test]
pub(crate) fn body_inputs_keep_nested_and_function_scopes() {
    let source = "n:3;again:=false;'outer{<A>:{-><uint8[n]>};f:(){<F>:{-><uint8[n]>}};'inner{<B>:{-><uint8[n]>};|again|'outer.restart();|again|'inner.restart()}}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let outer = checker.restart_inputs[&0].target;
    let inner = checker.restart_inputs[&1].target;
    let function = checker.functions[0].as_ref().unwrap().body.id;
    assert_eq!(checker.body_inputs.len(), 3);
    for id in [outer, inner, function] {
        assert_eq!(checker.body_inputs[&id].len(), 1);
        assert_eq!(checker.body_inputs[&id][0].id, 0);
    }
    assert_ne!(
        checker.bodies[&function].owner,
        checker.bodies[&outer].owner
    );
}

#[test]
pub(crate) fn body_input_capture_preserves_query_root_costs_and_source_failures() {
    let source = "p:@\"proof\";n:3;q:p.can_copy<({-><uint8[n]>})>()";
    let (checker, body) = check(source);
    let mut plain = Checker::new();
    for stmt in crate::parser::parse(source).unwrap().stmts {
        plain.stmt(&stmt).unwrap();
    }
    let a = checker.query_budgets[0].as_ref().unwrap();
    let b = plain.query_budgets[0].as_ref().unwrap();
    assert_eq!((a.steps, a.types, a.root), (b.steps, b.types, b.root));
    assert_eq!(checker.body_inputs[&body.id].len(), 1);
    assert_eq!(checker.body_inputs[&body.id][0].root, a.root);
    assert!(checker.type_work.is_none());
    for (source, code) in [
        ("n:1/0;<T>:{-><uint8[n]>}", "E107"),
        ("n:=3;<T>:{-><uint8[n]>}", "E211"),
    ] {
        let mut checker = Checker::new();
        let block = crate::parser::parse(source).unwrap();
        assert_eq!(checker.block(&block, None, None).unwrap_err().code, code);
        assert!(checker.body_inputs.is_empty());
        assert!(checker.type_work.is_none());
    }
}

#[test]
pub(crate) fn body_inputs_retain_lexical_control_without_backedge_propagation() {
    let source = "flag:false;n:3;again:=false;'loop{<Before>:{-><uint8[n]>};|flag|{<Inside>:{-><uint8[n]>}};|again|'loop.restart()};<After>:{-><uint8[n]>}";
    let mut checker = Checker::new();
    checker.derived.insert(0);
    let block = crate::parser::parse(source).unwrap();
    checker.block(&block, None, None).unwrap();
    let inputs = checker.body_inputs.values().flatten().collect::<Vec<_>>();
    assert_eq!(inputs.len(), 3);
    assert_eq!(inputs.iter().filter(|input| input.control).count(), 1);
    let target = checker.restart_inputs[&0].target;
    assert!(!checker.body_inputs[&target][0].control);
    assert!(!checker.control);
}

#[test]
pub(crate) fn body_input_capacity_and_derived_errors_preserve_prior_metadata() {
    for derived in [false, true] {
        let mut checker = Checker::new();
        super::super::tests::statements(&mut checker, "n:3");
        if derived {
            checker.mark_derived(0);
            checker.inputs.get_mut(&0).unwrap().derived = true;
        }
        checker.body_facts = MAX_BODY_FACTS;
        let block = crate::parser::parse("<T>:{-><uint8[n]>}").unwrap();
        let error = checker.block(&block, None, None).unwrap_err();
        assert_eq!(error.code, if derived { "E225" } else { "B001" });
        assert!(checker.body_inputs.is_empty());
        assert!(checker.type_work.is_none());
    }
}
