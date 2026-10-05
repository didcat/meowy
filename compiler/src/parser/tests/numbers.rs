use super::*;

#[test]
pub(crate) fn numeric_value_names_preserve_their_spelling_and_token_kind() {
    for name in ["1", "01", "0x1", "0Xf", "0b10", "1_000", "1e+2", "1e-2"] {
        let block = parse(&format!("{name}<int32>:=7;{name}=8;-> {name}:9")).unwrap();
        assert!(matches!(&block.stmts[0].kind,
            StmtKind::Bind { name: actual, mutable: true, .. } if actual == name));
        assert!(matches!(&block.stmts[1].kind,
            StmtKind::Assign { target, .. }
                if matches!(&target.kind, ExprKind::Int(actual) | ExprKind::Float(actual) if actual.text == name)));
        assert!(matches!(&block.stmts[2].kind,
            StmtKind::Emit { name: Some(actual), .. } if actual == name));
    }
}

#[test]
pub(crate) fn numeric_parameters_members_and_forward_names_parse() {
    for source in [
        "1<(int32)->int32>;1<int32>:(2<int32>){->2}",
        "f:(1<int32>,1e2<float64>){->1}",
        "row<{1<int32>;1e2<float64>}>:{->1:2;->1e2:3.0};row.1;row.1e2",
        "row.&1;row.&!0x1;row.*1e2",
    ] {
        parse(source).unwrap();
    }
}

#[test]
pub(crate) fn dotted_numeric_declarations_are_rejected_in_every_name_position() {
    for name in ["10.4", "1.0e+2", "1e2.4", "0x1.4"] {
        for source in [
            format!("{name}:1"),
            format!("{name}:=1"),
            format!("{name}<int32>:1"),
            format!("->{name}:1"),
            format!("f:({name}<int32>){{->1}}"),
            format!("row<{{{name}<int32>}}>:{{->1}}"),
        ] {
            assert_eq!(parse(&source).unwrap_err()[0].code, "E004", "{source}");
        }
    }
}

#[test]
pub(crate) fn numeric_member_tokens_form_individual_field_steps() {
    for source in ["row.1.0", "row . 1.0", "row.1 . 0", "row.1.0e-2"] {
        let field = value(source);
        let ExprKind::Field { value: inner, name } = field.kind else {
            panic!("{source}");
        };
        let expected = if source.ends_with("e-2") { "0e-2" } else { "0" };
        assert_eq!(name, expected, "{source}");
        assert_eq!(field.span, Span::new(0, source.len()));
        let ExprKind::Field { value: row, name } = inner.kind else {
            panic!("{source}");
        };
        assert_eq!(name, "1", "{source}");
        assert_eq!(inner.span, Span::new(0, source.find('1').unwrap() + 1));
        assert!(matches!(row.kind, ExprKind::Name(name) if name == "row"));
    }
    for op in ["&", "&!", "*"] {
        let ExprKind::Field { value: inner, name } = value(&format!("row.{op}1.0")).kind else {
            panic!();
        };
        assert_eq!(name, "0");
        let ExprKind::Unary { op: actual, value } = inner.kind else {
            panic!();
        };
        assert_eq!(actual, op);
        assert!(matches!(value.kind, ExprKind::Field { name, .. } if name == "1"));
    }
    let parsed = parse_documented_at("row.1.0", 30).unwrap();
    let StmtKind::Expr(field) = &parsed.block.stmts[0].kind else {
        panic!();
    };
    assert_eq!(field.span, Span::new(30, 37));
    let ExprKind::Field { value, .. } = &field.kind else {
        panic!();
    };
    assert_eq!(value.span, Span::new(30, 35));
}

#[test]
pub(crate) fn numeric_roots_retain_decimal_syntax_for_resolution_and_escape() {
    assert!(matches!(value("10.4").kind, ExprKind::Float(number) if number.text == "10.4"));
    let ExprKind::Call { args, .. } = value("@\"core\".literal(10.4)").kind else {
        panic!();
    };
    assert!(matches!(&args[0].kind, ExprKind::Float(number) if number.text == "10.4"));
    let block = parse("10.4<(int32)->int32>;").unwrap();
    assert!(matches!(&block.stmts[0].kind, StmtKind::Expr(expr)
        if matches!(expr.kind, ExprKind::Ascribe { predicate: true, .. })));
    for source in ["10.4.5", "10.4.name", "0x1.0x2", "1.0x2", "1e2.4"] {
        assert!(
            matches!(value(source).kind, ExprKind::Field { .. }),
            "{source}"
        );
    }
    assert_eq!(parse("10.").unwrap_err()[0].code, "E004");
    assert_eq!(
        parse(&format!("row{}", ".1.0".repeat(130))).unwrap_err()[0].code,
        "B001"
    );
}

#[test]
pub(crate) fn numeric_value_names_do_not_relax_tokens_types_or_labels() {
    for source in ["12cat:1", "1__0:1", "0x_ff:1"] {
        assert_eq!(parse(source).unwrap_err()[0].code, "E001");
    }
    for source in ["<1>:<int32>", "x<1>:2", "'1{}", "-1:2"] {
        assert_eq!(parse(source).unwrap_err()[0].code, "E004");
    }
}
