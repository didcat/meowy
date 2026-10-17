use super::{limits::snapshot, tests::checked, *};
use std::collections::BTreeSet;

#[test]
pub(crate) fn initializer_context_preserves_maps_after_hir_transfer_and_reordering() {
    for (case, source) in [
        "",
        "x:1;f<int32>:(a<int32>,b<int32>){v:a+b;->v};unused:(c<boolean>){};empty:(){}",
        "n:1;p:&n;r:{->p;->tag:true};copy:r;f:(p<&int32>,flag<boolean>){|flag|r:{->p;->tag:true};g<int32>:(n<int32>){->n}}",
        "f<(int32)->int32>;g<(int32)->int32>;g<int32>:(n<int32>){copy:n;->copy};f<int32>:(n<int32>){->g(n)};x:f(2)",
        "f<uint8>:(a<uint8>,unused<boolean>){copy:a;->copy};->alias<(uint8,boolean)->uint8>:f;x:alias(7,true)",
    ]
    .into_iter()
    .enumerate()
    {
        let (mut checker, mut program, reports) = checked(source);
        assert!(checker.functions.is_empty() && checker.locals.is_empty());
        let params = reports.signatures.params.keys().copied().collect::<BTreeSet<_>>();
        let start = checker.flow.work;
        assert_eq!(
            checker.program_signatures(&program, &reports, Span::default()).unwrap().params.keys().copied().collect::<BTreeSet<_>>(),
            params
        );
        let collection = checker.flow.work - start;
        let start = checker.flow.work;
        assert_eq!(
            checker.function_signatures(&program, &reports, Span::default()).unwrap(),
            reports.signatures
        );
        let headers = checker.flow.work - start;
        for local in &params {
            assert!(!reports.initializers.contains_key(local));
            assert!(!reports.shared_initializers.contains_key(local));
        }
        program.functions.reverse();
        let before = snapshot(&checker, &program, &reports);
        let start = checker.flow.work;
        assert_eq!(
            checker.binding_initializers(&program, &reports, Span::default()).unwrap(),
            reports.initializers
        );
        let ordinary = checker.flow.work - start;
        let start = checker.flow.work;
        let count = reports.shared_initializers.len();
        assert_eq!(
            checker.shared_initializers(&program, &reports, Span::default(), count, count).unwrap(),
            (reports.shared_initializers.clone(), 0)
        );
        let shared = checker.flow.work - start;
        assert_eq!(snapshot(&checker, &program, &reports), before);
        println!("case {case}: parameters {collection}, headers {headers}, ordinary {ordinary}, shared {shared}");
    }
}

#[test]
pub(crate) fn initializer_context_keeps_shared_parameter_and_special_cell_exclusions() {
    for state in 0..5 {
        let (mut checker, mut program, mut reports) =
            checked("n:1;p:&n;r:{->p;->tag:true};f:(x<int32>){}");
        let (&local, row) = reports.shared_initializers.first_key_value().unwrap();
        let statement = row.init.statement;
        match state {
            0 => {
                let function = &mut program.functions[0];
                checker.parameters.insert(
                    local,
                    crate::check::dependencies::Parameter {
                        function: function.id,
                        position: function.params.len(),
                        shape: crate::check::dependencies::bodies::completion::Completion::of(
                            &program.locals[local],
                        )
                        .result,
                    },
                );
                function.params.push(local);
                reports.signatures = checker
                    .function_signatures(&program, &reports, Span::default())
                    .unwrap();
            }
            1 => {
                let mut alias = checker.proofs.aliases.values().next().unwrap().clone();
                alias.root = local;
                checker.proofs.aliases.insert(local, alias);
            }
            2 => {
                checker.proofs.receivers.insert(local);
            }
            3 => {
                checker
                    .proofs
                    .temporaries
                    .insert(local, checker.points[statement].site.unwrap());
            }
            4 => {
                checker.proofs.mutable.insert(local);
            }
            _ => unreachable!(),
        }
        let before = snapshot(&checker, &program, &reports);
        let (index, parts) = checker
            .shared_initializers(&program, &reports, Span::default(), 1, 1)
            .unwrap();
        assert!(!index.contains_key(&local), "state {state}");
        assert_eq!(parts, 1);
        assert_eq!(snapshot(&checker, &program, &reports), before);
    }
}
