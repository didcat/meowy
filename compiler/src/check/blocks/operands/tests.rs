use super::*;

pub(super) fn expression(source: &str) -> ast::Expr {
    let StmtKind::Expr(expr) = crate::parser::parse(source).unwrap().stmts.remove(0).kind else {
        panic!()
    };
    expr
}

#[test]
pub(crate) fn operand_hints_preserve_existing_scalar_capture_and_exact_work() {
    let ty = Type::Int {
        signed: true,
        bits: 32,
    };
    for source in [
        "{->4}",
        "(({->4}))",
        "4.{->$}",
        "((4.{->$}))",
        "{->{->4}}",
        "{->(({->4}))}",
    ] {
        let expr = expression(source);
        let mut old = Checker::new();
        let mut new = Checker::new();
        let before = old.expression_point(&expr, Some(&ty)).unwrap();
        let after = new.operand_point(&expr, Some(&ty)).unwrap();
        assert_eq!(format!("{before:?}"), format!("{after:?}"), "{source}");
        assert_eq!(old.flow.work, new.flow.work, "{source}");
        assert_eq!(old.edge_counts(), new.edge_counts(), "{source}");
        assert_eq!(
            format!(
                "{:?}{:?}{:?}{:?}{:?}{:?}",
                old.points,
                old.coercions,
                old.group_inputs,
                old.dispatch_ops,
                old.bodies,
                old.sequences
            ),
            format!(
                "{:?}{:?}{:?}{:?}{:?}{:?}",
                new.points,
                new.coercions,
                new.group_inputs,
                new.dispatch_ops,
                new.bodies,
                new.sequences
            ),
            "{source}"
        );
        assert!(
            new.point.is_none() && new.frames.is_empty() && new.scopes.len() == old.scopes.len()
        );
    }
}

#[test]
pub(crate) fn operand_hints_preserve_required_errors_stops_and_exhaustion() {
    let ty = Type::Int {
        signed: true,
        bits: 32,
    };
    for source in ["{->1;->tag:true}", "((1.{->$;->tag:true}))"] {
        let mut checker = Checker::new();
        checker.required = true;
        let error = checker
            .operand_point(&expression(source), Some(&ty))
            .unwrap_err();
        assert_eq!(error.code, "E207");
        assert!(checker.point.is_none());
    }
    for (source, code) in [
        ("n:=1;x:1+(&!n).{->1;->tag:true}", "B001"),
        ("x:1+3.{$=4;->$;->tag:true}", "E305"),
        ("x:1+{->missing;->tag:true}", "E201"),
        ("d:@\"debug\";x:1+{d.panic(\"stop\");->tag:missing}", "E201"),
        ("x:true&&{->true;->tag:true}", "E207"),
        ("x:false||((true.{->$;->tag:true}))", "E207"),
        (
            "x:1+'outer{'inner{'outer->tag:true;'inner.restart()};->2}",
            "B001",
        ),
    ] {
        let errors = crate::compile(source).unwrap_err();
        assert_eq!(errors[0].code, code, "{source}: {errors:?}");
        if code == "B001" {
            assert!(errors[0].message.contains(if source.contains("restart") {
                "restart after an emission into an enclosing scope"
            } else {
                "exclusive dispatch receivers"
            }));
        }
    }
    let mut checker = Checker::new();
    assert!(!checker.flow.spend(usize::MAX));
    let error = checker
        .operand_point(&expression("((3.{->$;->tag:true}))"), Some(&ty))
        .unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("continuation expression budget"));
    assert!(checker.point.is_none() && checker.points.is_empty());
    assert!(checker.frames.is_empty() && checker.proofs.receivers.is_empty());
}
