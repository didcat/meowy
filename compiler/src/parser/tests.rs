use super::*;
use crate::ast::*;

mod borrows;
mod doc_fences;
mod subtraction;

pub(crate) fn value(source: &str) -> Expr {
    let block = parse(source).unwrap();
    match block.stmts.into_iter().next().unwrap().kind {
        StmtKind::Bind { value, .. } | StmtKind::Expr(value) => value,
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
pub(crate) fn preserves_grouped_integer_negation() {
    let ExprKind::Unary { value: raw, .. } = value("x<int8>:-128").kind else {
        panic!()
    };
    assert!(matches!(raw.kind, ExprKind::Int(_)));
    let ExprKind::Unary { value: grouped, .. } = value("x<int8>:-(128)").kind else {
        panic!()
    };
    assert!(matches!(grouped.kind, ExprKind::Group(_)));
}

#[test]
pub(crate) fn parses_functions_labels_and_emissions() {
    let source = "even<(uint32)->boolean>;even<boolean>:(n<uint32>) 'answer {|n==0|{'answer->true;'answer.leave()};->even(n-1)}";
    let block = parse(source).unwrap();
    assert_eq!(block.stmts.len(), 2);
    assert!(matches!(block.stmts[0].kind, StmtKind::Forward { .. }));
    let StmtKind::Bind { value, .. } = &block.stmts[1].kind else {
        panic!()
    };
    let ExprKind::Function { body, .. } = &value.kind else {
        panic!()
    };
    assert_eq!(body.label.as_deref(), Some("answer"));
    assert!(matches!(body.stmts[1].kind, StmtKind::Emit { .. }));
}

#[test]
pub(crate) fn honors_context_for_type_suffixes() {
    let block = parse("|!(value<int32>) && accepts(value<int32>)|copy:value<int32>").unwrap();
    let StmtKind::Match { arms } = &block.stmts[0].kind else {
        panic!()
    };
    let ExprKind::Binary { left, right, .. } = &arms[0].0.as_ref().unwrap().kind else {
        panic!()
    };
    let ExprKind::Unary { value, .. } = &left.kind else {
        panic!()
    };
    let ExprKind::Group(value) = &value.kind else {
        panic!()
    };
    assert!(matches!(
        value.kind,
        ExprKind::Ascribe {
            predicate: true,
            ..
        }
    ));
    let ExprKind::Call { args, .. } = &right.kind else {
        panic!()
    };
    assert!(matches!(
        args[0].kind,
        ExprKind::Ascribe {
            predicate: false,
            ..
        }
    ));
    let StmtKind::Bind { value, .. } = &arms[0].1.kind else {
        panic!()
    };
    assert!(matches!(
        value.kind,
        ExprKind::Ascribe {
            predicate: false,
            ..
        }
    ));
}

#[test]
pub(crate) fn gives_predicates_comparison_precedence() {
    let block = parse("|!value<boolean>|f()").unwrap();
    let StmtKind::Match { arms } = &block.stmts[0].kind else {
        panic!()
    };
    let ExprKind::Ascribe {
        value,
        predicate: true,
        ..
    } = &arms[0].0.as_ref().unwrap().kind
    else {
        panic!()
    };
    assert!(matches!(value.kind, ExprKind::Unary { .. }));
    assert!(parse("x:1<2<3").is_err());
    assert!(parse("|left<limit&&other>0|f()").is_ok());
    assert_eq!(parse("left<limit<other").unwrap_err()[0].code, "E004");
    let ExprKind::Binary { left, op, .. } = self::value("copy:value<int32><2").kind else {
        panic!()
    };
    assert_eq!(op, "<");
    assert!(matches!(
        left.kind,
        ExprKind::Ascribe {
            predicate: false,
            ..
        }
    ));
}

#[test]
pub(crate) fn parses_dispatch_and_continuation() {
    let expr = value("answer:20\n.(increment)\n.(double,\n2)");
    let ExprKind::Dispatch { value, args, .. } = expr.kind else {
        panic!()
    };
    assert_eq!(args.len(), 1);
    assert!(matches!(value.kind, ExprKind::Dispatch { .. }));
    assert!(parse("x:1+\n2\ny:(1\n+2)").is_ok());
    assert!(parse("x:1 y:2").is_err());
}

#[test]
pub(crate) fn parses_record_annotations_and_computed_types() {
    assert!(parse("<Reading>:<{-><int32>;unit<string>;count<uint32>:=}>;x<Reading>:{->2;->unit:\"c\";->count:=0}").is_ok());
    assert!(parse("other<(name<>)>:value").is_ok());
    assert!(parse("other<(element)[capacity]>:value").is_ok());
    let block = parse("<Bytes>:bounded(<uint8>,4)").unwrap();
    let StmtKind::TypeAlias { ty, .. } = &block.stmts[0].kind else {
        panic!()
    };
    assert!(matches!(ty.kind, TypeKind::Computed(_)));
}

#[test]
pub(crate) fn interpolation_has_absolute_spans_and_nested_strings() {
    let expr = value("text:\"a {f(\"b {2}\")} z\"");
    let ExprKind::String(parts) = expr.kind else {
        panic!()
    };
    let StringPart::Value(expr) = &parts[1] else {
        panic!()
    };
    assert_eq!(expr.span.start, 9);
    assert!(matches!(expr.kind, ExprKind::Call { .. }));
    let errors = parse("text:\"bad {1 + }\"").unwrap_err();
    assert_eq!(errors[0].span.start, 15);
}

#[test]
pub(crate) fn rejects_unavailable_generics_and_malformed_delimiters() {
    assert_eq!(parse("f<:T>:(x<T>){->x}").unwrap_err()[0].code, "B001");
    assert_eq!(crate::compile("f<int32>(1)").unwrap_err()[0].code, "B001");
    assert_eq!(parse("bytes.filled<4>(0)").unwrap_err()[0].code, "B001");
    assert_eq!(parse("x:(1]").unwrap_err()[0].code, "E002");
    assert!(parse("|true|f();||g()").is_err());
}

#[test]
pub(crate) fn bounds_nesting_without_panicking() {
    let source = format!("x:{}1{}", "(".repeat(80), ")".repeat(80));
    assert!(
        parse(&source)
            .unwrap_err()
            .iter()
            .any(|error| error.code == "B001")
    );
    let source = format!("{}f()", "|true| ".repeat(80));
    assert!(
        parse(&source)
            .unwrap_err()
            .iter()
            .any(|error| error.code == "B001")
    );
}

#[test]
pub(crate) fn bounds_tree_depth_for_flat_operator_chains() {
    assert!(parse(&format!("x:{}", vec!["true"; 80].join("||"))).is_ok());
    for source in [
        format!("x:{}", vec!["1"; 20_000].join("+")),
        format!("x:{}", vec!["value"; 20_000].join(".")),
    ] {
        assert_eq!(parse(&source).unwrap_err()[0].code, "B001");
    }
}

#[test]
pub(crate) fn malformed_unicode_inputs_preserve_lexer_spans_without_panics() {
    let symbols = [
        "a", "1", "0x", " ", "\n", "\r", "<", ">", "(", ")", "{", "}", "[", "]", "|", "!", "&",
        ";", ":", "=", "+", ".", "'", "@", "\"", "\\", "#", "é", "💖",
    ];
    let mut seed = 0x59eedu64;
    for case in 0..10_000 {
        let mut source = String::new();
        for _ in 0..case % 96 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            source.push_str(symbols[(seed >> 32) as usize % symbols.len()]);
        }
        let _ = parse(&source);
        if let Ok(tokens) = lexer::lex(&source) {
            assert_eq!(
                tokens
                    .iter()
                    .map(|token| token.text.as_str())
                    .collect::<String>(),
                source
            );
            for token in tokens {
                assert_eq!(
                    source.get(token.span.start..token.span.end),
                    Some(token.text.as_str())
                );
            }
        }
    }
}

#[test]
pub(crate) fn exported_types_parse_distinct_namespace_flags_and_shifted_spans() {
    let source = "<Private>:<int32>;-><Public>:<{n<Private>}>;->value:7";
    let block = parse_documented_at(source, 100).unwrap().block;
    assert!(matches!(
        &block.stmts[0].kind,
        StmtKind::TypeAlias {
            exported: false,
            ..
        }
    ));
    let StmtKind::TypeAlias {
        name,
        ty,
        exported: true,
    } = &block.stmts[1].kind
    else {
        panic!("exported type")
    };
    assert_eq!(name, "Public");
    assert!(matches!(ty.kind, TypeKind::Record { .. }));
    assert_eq!(
        block.stmts[1].span.start,
        100 + source.find("-><Public>").unwrap()
    );
    assert!(matches!(block.stmts[2].kind, StmtKind::Emit { .. }));
    let block = parse("-><Alias>:core.int32").unwrap();
    assert!(matches!(
        &block.stmts[0].kind,
        StmtKind::TypeAlias {
            exported: true,
            ty: TypeExpr {
                kind: TypeKind::Computed(_),
                ..
            },
            ..
        }
    ));
    assert!(crate::compile("-><T>:<int32>").is_ok());
}

#[test]
pub(crate) fn exported_types_keep_invalid_and_labeled_declarations_gated() {
    assert!(parse("-><T>:=<int32>").is_err());
    assert!(parse("-><T><U>:<int32>").is_err());
    assert_eq!(parse("-><m.T>:<int32>").unwrap_err()[0].code, "B001");
    assert_eq!(
        parse("'out{'out-><T>:<int32>}").unwrap_err()[0].code,
        "B001"
    );
}

#[test]
pub(crate) fn explicit_type_calls_retain_arguments_and_source_spans() {
    let source = "f<uint32, &int32[2]>(7, 8)";
    let ExprKind::Call { callee, args } = value(source).kind else {
        panic!()
    };
    assert_eq!(args.len(), 2);
    assert_eq!(&source[args[0].span.start..args[0].span.end], "7");
    let ExprKind::Specialize {
        value: callee,
        types,
    } = callee.kind
    else {
        panic!()
    };
    assert!(matches!(callee.kind, ExprKind::Name(ref name) if name == "f"));
    assert_eq!(types.len(), 2);
    assert_eq!(&source[types[0].span.start..types[0].span.end], "uint32");
    assert_eq!(&source[types[1].span.start..types[1].span.end], "&int32[2]");
    for source in [
        "f < uint32 > ()",
        "p.can_copy<uint32>()",
        "f<{n<int32>}>()",
        "f<{n<int32>},uint32>()",
        "f<(kind)>()",
    ] {
        let ExprKind::Call { callee, .. } = value(source).kind else {
            panic!("{source}")
        };
        assert!(
            matches!(callee.kind, ExprKind::Specialize { .. }),
            "{source}"
        );
    }
}

#[test]
pub(crate) fn explicit_type_calls_preserve_suffix_context_and_limits() {
    assert!(matches!(value("x<uint32>").kind, ExprKind::Ascribe { .. }));
    assert!(matches!(value("x<>").kind, ExprKind::TypeQuery(_)));
    assert!(matches!(value("x < y").kind, ExprKind::Binary { ref op, .. } if op == "<"));
    assert!(matches!(
        value("f<uint32>()<>").kind,
        ExprKind::TypeQuery(_)
    ));
    let args = std::iter::repeat_n("int32", 64)
        .collect::<Vec<_>>()
        .join(",");
    assert!(parse(&format!("f<{args}>()")).is_ok());
    assert_eq!(
        parse(&format!("f<{args},int32>()")).unwrap_err()[0].code,
        "B001"
    );
    assert_eq!(
        parse(&format!("f{}", "<int32>()".repeat(140))).unwrap_err()[0].code,
        "B001"
    );
    assert_eq!(parse("f<int32><null>()").unwrap_err()[0].code, "B001");
    assert!(parse("f<int32,>()").is_err());
}

#[test]
pub(crate) fn explicit_ascription_consumes_one_target_before_predicates() {
    for source in [
        "|value~<T><U>|matched()",
        "| value ~ < T > < U > | matched()",
    ] {
        let block = parse(source).unwrap();
        let StmtKind::Match { arms } = &block.stmts[0].kind else {
            panic!()
        };
        let ExprKind::Ascribe {
            value,
            predicate: true,
            ..
        } = &arms[0].0.as_ref().unwrap().kind
        else {
            panic!()
        };
        assert!(matches!(
            value.kind,
            ExprKind::Ascribe {
                predicate: false,
                ..
            }
        ));
    }
}

#[test]
pub(crate) fn explicit_ascription_preserves_generic_calls_and_type_queries() {
    let block = parse("out:f<T>(value~<T>);ty:value~<T><>").unwrap();
    let StmtKind::Bind { value, .. } = &block.stmts[0].kind else {
        panic!()
    };
    let ExprKind::Call { callee, args } = &value.kind else {
        panic!()
    };
    assert!(matches!(callee.kind, ExprKind::Specialize { .. }));
    assert!(matches!(
        args[0].kind,
        ExprKind::Ascribe {
            predicate: false,
            ..
        }
    ));
    let StmtKind::Bind { value, .. } = &block.stmts[1].kind else {
        panic!()
    };
    assert!(matches!(value.kind, ExprKind::TypeQuery(_)));
    assert!(parse("out:value~T").is_err());
}
