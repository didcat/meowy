use super::*;
use crate::check::dependencies::{
    ScalarLiteral,
    edges::forward::{effects::Effect, entries::tests::checked},
};

#[test]
pub(crate) fn scalar_seeds_ignore_frontend_caches_and_do_not_invent_read_bind_or_call_values() {
    let source = "a:7;copy:a;f<int32>:(n<int32>){->n};result:f(1)";
    crate::compile(source).unwrap();
    let (mut checker, program) = checked(source);
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let (before, _, _) = checker
        .scalar_seed_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let expected = before
        .iter()
        .map(|(point, seed)| (*point, (seed.declared.clone(), seed.value.clone())))
        .collect::<BTreeMap<_, _>>();
    checker.constants.clear();
    checker.inputs.clear();
    checker.bool_inputs.clear();
    checker.record_inputs.clear();
    checker.bools.clear();
    checker.tags.clear();
    checker.pointees.clear();
    let (after, _, _) = checker
        .scalar_seed_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let actual = after
        .iter()
        .map(|(point, seed)| (*point, (seed.declared.clone(), seed.value.clone())))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(actual, expected);
    assert_eq!(after.len(), 2);
    for point in checker
        .local_reads
        .keys()
        .chain(checker.operations.keys())
        .copied()
        .chain(checker.invocations.values().map(|call| call.point))
    {
        assert!(!after.contains_key(&point));
    }
    assert!(checker.constants.is_empty() && checker.bools.is_empty() && checker.tags.is_empty());
}

#[test]
pub(crate) fn scalar_seeds_preserve_control_marks_and_omit_stopped_successors() {
    let source =
        "seed:false;d:@\"debug\";|seed|{n:7};stop<never>:(){d.panic(\"stop\")};stop();late:9";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    checker.derived.insert(0);
    let ast = crate::parser::parse(source).unwrap();
    let body = checker.block(&ast, None, None).unwrap();
    let program = hir::Program {
        body,
        functions: std::mem::take(&mut checker.functions)
            .into_iter()
            .flatten()
            .collect(),
        locals: std::mem::take(&mut checker.locals),
    };
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let marks = checker.derived.clone();
    let (seeds, _, _) = checker
        .scalar_seed_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(seeds.values().any(|seed| matches!(&seed.source.1, Effect::Scalar(value) if value.control && value.value == Some(ScalarLiteral::Int(7)))));
    let late = checker
        .scalar_leaves
        .iter()
        .find(|(_, leaf)| leaf.value == Some(ScalarLiteral::Int(9)))
        .unwrap()
        .0;
    assert!(!seeds.contains_key(late));
    assert_eq!(checker.derived, marks);
}
