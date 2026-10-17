use super::{super::entries::tests::checked, *};
use crate::check::dependencies::ScalarKind;

#[test]
pub(crate) fn declared_parameter_shapes_follow_checked_types_and_original_call_pairs() {
    let int = ScalarKind::Int {
        bits: 32,
        signed: true,
    };
    for (source, shapes) in [
        (
            "f:(a<uint8>,b<float64>,c<boolean>,d<string>,e<null>){};f(1,2.0,true,\"s\",null)",
            vec![
                Shape::Scalar(ScalarKind::Int {
                    bits: 8,
                    signed: false,
                }),
                Shape::Scalar(ScalarKind::Float { bits: 64 }),
                Shape::Scalar(ScalarKind::Bool),
                Shape::Scalar(ScalarKind::String),
                Shape::Scalar(ScalarKind::Null),
            ],
        ),
        (
            "f:(a<&int32>,b<&{n<int32>}>){};n:1;r:{->n:2};f(&n,&r)",
            vec![
                Shape::SharedScalar(int),
                Shape::Reference(hir::ReferenceMode::Shared),
            ],
        ),
        (
            "f:(a<&!int32>,unused<boolean>){};n:=1;f(&!n,true)",
            vec![
                Shape::Reference(hir::ReferenceMode::Exclusive),
                Shape::Scalar(ScalarKind::Bool),
            ],
        ),
        (
            "f:(a<{n<int32>}>,b<int32[2]>,c<int32><null>){};r:{->n:1};f(r,[2,3],null)",
            vec![
                Shape::Record { fields: 1 },
                Shape::List { capacity: 2 },
                Shape::Union { members: 2 },
            ],
        ),
    ] {
        crate::compile(source).unwrap();
        let (mut checker, program) = checked(source);
        let reports = checker.entry_reports(&program, Span::default()).unwrap();
        assert!(checker.locals.is_empty() && checker.functions.is_empty());
        let call = checker.invocations.first_key_value().unwrap().1.clone();
        let header = &reports.signatures.functions[&call.function];
        let actual = header
            .params
            .iter()
            .map(|&local| Completion::of(&program.locals[local]).result)
            .collect::<Vec<_>>();
        assert_eq!(actual, shapes, "{source}");
        let (forest, _) = checker
            .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap();
        let (_, inputs) = checker
            .call_argument_forest(&forest, call.site, Span::default())
            .unwrap();
        for (position, (param, point, row, marker)) in inputs.enumerate() {
            assert_eq!(point, call.args[position]);
            assert_eq!(
                Completion::of(&program.locals[param]).result,
                shapes[position]
            );
            assert_eq!(marker.is_some(), row.is_some_and(|row| row.normal));
            assert!(!reports.initializers.contains_key(&param));
        }
    }
}

#[test]
pub(crate) fn declared_parameter_shapes_survive_unused_stopped_and_shadowed_inputs() {
    for source in [
        "f:(a<&int32>,b<int32[2]>){}",
        "d:@\"debug\";f:(a<int32>,b<&int32>){};n:1;f(d.panic(\"stop\"),&n)",
        "n:2;f:(n<int32>,items<int32[n]>){}",
        "f<(uint8)->null>;f<null>:(n<uint8>){};f(1)",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, program) = checked(source);
        let reports = checker.entry_reports(&program, Span::default()).unwrap();
        for function in &program.functions {
            let header = &reports.signatures.functions[&function.id];
            for (position, &local) in header.params.iter().enumerate() {
                let shape = Completion::of(&program.locals[local]).result;
                assert!(
                    Completion {
                        normal: shape != Shape::Never,
                        result: shape
                    }
                    .valid()
                );
                assert_eq!(reports.signatures.params[&local], (function.id, position));
            }
        }
        if source.starts_with("d:") {
            assert!(reports.argument_sources.is_empty());
            assert!(reports.argument_walk.visits.is_empty());
            assert_eq!(reports.signatures.params.len(), 2);
        }
        if source.starts_with("n:") {
            let local = program.functions[0].params[1];
            assert_eq!(
                Completion::of(&program.locals[local]).result,
                Shape::List { capacity: 2 }
            );
        }
    }
}
