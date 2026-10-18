use super::*;
use crate::check::dependencies::edges::forward::entries::tests::checked;

pub(super) fn context(source: &str) -> (Checker, hir::Program, Reports) {
    crate::compile(source).unwrap();
    let (mut checker, program) = checked(source);
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    (checker, program, reports)
}

#[test]
pub(crate) fn scalar_declarations_borrow_full_types_and_preserve_original_parameter_positions() {
    let source = "f<uint8>:(n<uint8>,flag<boolean>,maybe<int16><null>,view<&int32>){->7};g<boolean>:(){->false};unused<int32><null>:(value<int32><null>){->null};x:1;f(7,true,2,&x)";
    let (mut checker, mut program, reports) = context(source);
    program.functions.reverse();
    let (seeds, _) = checker
        .scalar_declared_seeds(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    for function in &program.functions {
        let result = &seeds.results[&function.id];
        assert!(std::ptr::eq(result.ty, &function.result));
        for (position, local) in function.params.iter().enumerate() {
            let (owner, found, value) = &seeds.params[local];
            assert_eq!((*owner, *found), (function.id, position));
            assert!(std::ptr::eq(value.ty, &program.locals[*local]));
        }
        if function.name == "f" {
            assert_ne!(
                result.domain,
                Some(Domain::integer(7, &mut checker.flow, Span::default()).unwrap())
            );
            assert_eq!(result.domain, seeds.params[&function.params[0]].2.domain);
            assert!(seeds.params[&function.params[3]].2.domain.is_none());
        }
        if function.name == "g" {
            assert_ne!(result.domain, Some(Domain::boolean(false)));
        }
        if function.name == "unused" {
            let domain = result.domain.as_ref().unwrap();
            assert!(domain.null && domain.values.is_some());
        }
    }
}

#[test]
pub(crate) fn scalar_declarations_distinguish_equal_shallow_union_shapes_and_never_results() {
    let source = "f:(a<boolean><null>,b<int32><null>,opaque<boolean><int32>){};d:@\"debug\";stop<int32>:(){d.panic(\"stop\")};never<never>:(){d.panic(\"stop\")}";
    let (mut checker, program, reports) = context(source);
    let function = &program.functions[0];
    let shapes = &reports.signatures.functions[&function.id].inputs;
    assert!(shapes.iter().all(|shape| *shape == shapes[0]));
    let (seeds, _) = checker
        .scalar_declared_seeds(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let first = &seeds.params[&function.params[0]].2.domain;
    let second = &seeds.params[&function.params[1]].2.domain;
    assert!(first.is_some() && second.is_some() && first != second);
    assert!(seeds.params[&function.params[2]].2.domain.is_none());
    assert_ne!(
        seeds.results[&program.functions[1].id].domain,
        Some(Domain::BOTTOM)
    );
    assert_eq!(
        seeds.results[&program.functions[2].id].domain,
        Some(Domain::BOTTOM)
    );
}

#[test]
pub(crate) fn scalar_seed_context_shares_declaration_literal_payload_and_exact_work() {
    let (mut checker, program, reports) =
        context("f<uint8>:(n<uint8>,flag<boolean>){->7};f(1,true)");
    let start = checker.flow.work;
    let (literals, declared, left) = checker
        .scalar_seed_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let cost = checker.flow.work - start;
    let parts = MAX_EDGES - left;
    assert_eq!(literals.len(), 3);
    assert_eq!(declared.params.len(), 2);
    assert_eq!(declared.results.len(), 1);
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for (limit, extra, pass) in [(parts - 1, 0, false), (parts, 1, false), (parts, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        assert_eq!(
            checker
                .scalar_seed_context(&program, &reports, Span::default(), limit)
                .is_ok(),
            pass
        );
        assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn scalar_declared_seeds_reject_stale_signature_and_local_identities() {
    for fault in 0..4 {
        let (mut checker, mut program, mut reports) =
            context("f<uint8>:(a<int32>,b<int32>){->7};g:(){}");
        match fault {
            0 => {
                program.locals.pop();
            }
            1 => program.functions[0].params.reverse(),
            2 => program.functions[0].id = program.functions[1].id,
            3 => {
                reports
                    .signatures
                    .functions
                    .get_mut(&program.functions[0].id)
                    .unwrap()
                    .shape = crate::check::dependencies::Invocation::result_shape(&hir::Type::Bool)
            }
            _ => unreachable!(),
        }
        let before = format!("{program:?}{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .scalar_seed_context(&program, &reports, Span::default(), MAX_EDGES)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(
            before,
            format!("{program:?}{reports:?}{:?}", checker.edge_counts())
        );
    }
}
