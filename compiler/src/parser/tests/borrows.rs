use super::*;

pub(crate) fn tree(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Name(name) | ExprKind::Int(name) => name.clone(),
        ExprKind::Unary { op, value } => format!("({op} {})", tree(value)),
        ExprKind::Group(value) => format!("(group {})", tree(value)),
        ExprKind::Field { value, name } => format!("(field {} {name})", tree(value)),
        ExprKind::Index { value, index } => format!("(index {} {})", tree(value), tree(index)),
        ExprKind::Call { callee, args } => {
            assert!(args.is_empty());
            format!("(call {})", tree(callee))
        }
        ExprKind::Dispatch {
            value,
            callee,
            args,
        } => {
            assert!(args.is_empty());
            format!("(dispatch {} {})", tree(value), tree(callee))
        }
        ExprKind::TypeQuery(value) => format!("(query {})", tree(value)),
        ExprKind::Ascribe {
            value, predicate, ..
        } => format!("(ascribe {predicate} {})", tree(value)),
        other => panic!("unexpected expression: {other:?}"),
    }
}

#[test]
pub(crate) fn borrows_before_postfix_selection() {
    for op in ["&", "&!", "*"] {
        for (source, expected) in [
            (format!("{op}items[i]"), format!("(index ({op} items) i)")),
            (
                format!("{op}(items[i])"),
                format!("({op} (group (index items i)))"),
            ),
            (format!("{op}r.field"), format!("(field ({op} r) field)")),
            (format!("r.{op}field"), format!("({op} (field r field))")),
            (
                format!("{op}(r.field)"),
                format!("({op} (group (field r field)))"),
            ),
            (format!("{op}f()"), format!("(call ({op} f))")),
            (format!("{op}(f())"), format!("({op} (group (call f)))")),
            (format!("{op}r.(f)"), format!("(dispatch ({op} r) f)")),
        ] {
            assert_eq!(tree(&value(&source)), expected, "{source}");
        }
    }
}

#[test]
pub(crate) fn dotted_borrows_select_one_field() {
    for (source, expected) in [
        ("r.&inner.field", "(field (& (field r inner)) field)"),
        ("r.inner.&field", "(& (field (field r inner) field))"),
        ("r.&items[i]", "(index (& (field r items)) i)"),
        ("&(r.items[i])", "(& (group (index (field r items) i)))"),
        (
            "r.items[i].&!field",
            "(&! (field (index (field r items) i) field))",
        ),
        ("&r.&!field", "(&! (field (& r) field))"),
        ("&(r.&!field)", "(& (group (&! (field r field))))"),
        ("r.&field()", "(call (& (field r field)))"),
        ("r.&field.(f)", "(dispatch (& (field r field)) f)"),
        ("r.*field", "(* (field r field))"),
        ("r.inner.*field", "(* (field (field r inner) field))"),
        ("r.*field.next", "(field (* (field r field)) next)"),
        ("r.*items[i]", "(index (* (field r items)) i)"),
        ("r.*field()", "(call (* (field r field)))"),
    ] {
        assert_eq!(tree(&value(source)), expected, "{source}");
    }
}

#[test]
pub(crate) fn borrow_prefix_chains_stop_before_postfix() {
    for (source, expected) in [
        ("&*p", "(& (* p))"),
        ("&!*p", "(&! (* p))"),
        ("&*p.field", "(field (& (* p)) field)"),
        ("&!*p[i]", "(index (&! (* p)) i)"),
        ("&*f()", "(call (& (* f)))"),
        ("&*(p.field)", "(& (* (group (field p field))))"),
        ("*p.field", "(field (* p) field)"),
        ("*(p.field)", "(* (group (field p field)))"),
        ("*r.&field", "(& (field (* r) field))"),
        ("*&r.field", "(field (* (& r)) field)"),
        ("**r.field", "(field (* (* r)) field)"),
        ("*-r.field", "(field (* (- r)) field)"),
        ("&-r.field", "(field (& (- r)) field)"),
        ("-r.field", "(- (field r field))"),
        ("& &r.field", "(field (& (& r)) field)"),
        ("&! &r.field", "(field (&! (& r)) field)"),
        ("&!r.field", "(field (&! r) field)"),
        ("&(!r.field)", "(& (group (! (field r field))))"),
    ] {
        assert_eq!(tree(&value(source)), expected, "{source}");
    }
}

#[test]
pub(crate) fn borrow_type_suffixes_apply_to_the_reference() {
    for (source, expected) in [
        ("&x<int32>", "(ascribe false (& x))"),
        ("&(x<int32>)", "(& (group (ascribe false x)))"),
        ("&x<>", "(query (& x))"),
        ("r.&field<&int32>", "(ascribe false (& (field r field)))"),
        ("r.&field<>", "(query (& (field r field)))"),
        ("*x<int32>", "(ascribe false (* x))"),
        ("*(x<&int32>)", "(* (group (ascribe false x)))"),
        ("r.*field<>", "(query (* (field r field)))"),
    ] {
        assert_eq!(tree(&value(source)), expected, "{source}");
    }
    let block = parse("|&x<&int32>|f()").unwrap();
    let StmtKind::Match { arms } = &block.stmts[0].kind else {
        panic!()
    };
    assert_eq!(tree(arms[0].0.as_ref().unwrap()), "(ascribe true (& x))");
    let ExprKind::Call { callee, .. } = value("r.&f<int32>()").kind else {
        panic!()
    };
    let ExprKind::Specialize { value, .. } = callee.kind else {
        panic!()
    };
    assert_eq!(tree(&value), "(& (field r f))");
    assert_eq!(crate::compile("r.&f<int32>()").unwrap_err()[0].code, "B001");
}

#[test]
pub(crate) fn borrow_continuations_keep_absolute_spans() {
    for (source, expected) in [
        ("&r\n.field", "(field (& r) field)"),
        ("r\n.&!\nfield", "(&! (field r field))"),
        ("r.\n&\nfield", "(& (field r field))"),
        ("&\nitems[i]", "(index (& items) i)"),
        ("r\n.*\nfield", "(* (field r field))"),
    ] {
        assert_eq!(tree(&value(source)), expected, "{source}");
    }
    let source = "text:\"é🙂 {row.&!field}\"";
    let ExprKind::String(parts) = value(source).kind else {
        panic!()
    };
    let StringPart::Value(expr) = &parts[1] else {
        panic!()
    };
    let ExprKind::Unary { value: field, .. } = &expr.kind else {
        panic!()
    };
    assert_eq!(tree(expr), "(&! (field row field))");
    assert_eq!(expr.span.start, source.find("row").unwrap());
    assert_eq!(&source[expr.span.start..expr.span.end], "row.&!field");
    assert_eq!(field.span, expr.span);
    let ExprKind::Field { value: row, .. } = &field.kind else {
        panic!()
    };
    assert_eq!(&source[row.span.start..row.span.end], "row");
}

#[test]
pub(crate) fn dotted_borrows_require_a_field_name() {
    for source in [
        "r.&",
        "r.&!",
        "r.&(field)",
        "r.&[i]",
        "r.&!{->1}",
        "r.&!&field",
        "r.& !field",
        "r.&.field",
        "r.*",
        "r.*(field)",
        "r.*[i]",
        "r.*!field",
    ] {
        assert_eq!(parse(source).unwrap_err()[0].code, "E004", "{source}");
    }
    let source = "text:\"é🙂 {r.&!}\"";
    let errors = parse(source).unwrap_err();
    assert_eq!(errors[0].code, "E004");
    assert_eq!(errors[0].span.start, source.find('}').unwrap());
}

#[test]
pub(crate) fn borrow_chains_respect_expression_and_tree_budgets() {
    assert!(parse(&format!("x:r{}", ".&field".repeat(80))).is_ok());
    for source in [
        format!("x:r{}", ".&field".repeat(20_000)),
        format!("x:{}r", "& ".repeat(80)),
        format!("x:&{}p.field", "*".repeat(80)),
    ] {
        assert_eq!(parse(&source).unwrap_err()[0].code, "B001");
    }
}
