use crate::ast::Span;
use crate::check::Checker;

#[test]
pub(crate) fn checked_signatures_reuse_parameter_types_before_body_shadowing() {
    for source in [
        "n:2;f<int32>:(n<int32>,items<int32[n]>){->items[1]};v:f(3,[7,9])",
        "n:2;f<(int32,int32[2])->int32>;f<int32>:(n<int32>,items<int32[n]>){->items[1]};v:f(3,[7,9])",
    ] {
        let program = crate::compile(source).unwrap();
        let function = &program.functions[0];
        assert_eq!(
            program.locals[function.params[1]],
            crate::hir::Type::List {
                element: Box::new(crate::hir::Type::Int {
                    bits: 32,
                    signed: true
                }),
                capacity: 2,
            }
        );
    }
    for (source, code) in [
        ("f<int32>:(n<int32>,n<int32>){->n}", "E203"),
        ("f<int32>:(items<int32[1/0]>){->0}", "E107"),
        ("f<(int32)->int32>;f<int32>:(n<uint8>){->0}", "E221"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn checked_signatures_do_not_replay_computed_parameter_work() {
    let block = crate::parser::parse("f<int32>:(n<({-><int32>})>){->n}").unwrap();
    let mut checker = Checker::new();
    checker
        .required_root(Span::new(100, 150), |checker| {
            checker.block(&block, None, None)?;
            let work = checker.type_work.as_ref().unwrap();
            assert_eq!((work.visits, work.nodes), (3, 2));
            assert_eq!((work.logical.steps, work.logical.types), (5, 2));
            Ok(())
        })
        .unwrap();
}
