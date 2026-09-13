use super::*;
use crate::ast::{Expr, TypeExpr, TypeKind};

pub(crate) fn literal() -> Expr {
    Expr {
        kind: ExprKind::TypeValue(TypeExpr {
            kind: TypeKind::Name("int32".into()),
            span: Span::new(3, 10),
        }),
        span: Span::new(3, 10),
    }
}

#[test]
pub(crate) fn computed_types_bound_nested_depth_and_reset_failed_roots() {
    let mut expr = literal();
    for _ in 0..MAX_DEPTH {
        expr = Expr {
            kind: ExprKind::Group(Box::new(expr)),
            span: Span::new(1, 12),
        };
    }
    let mut checker = Checker::new();
    let error = checker.type_value(&expr).unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("computed type bootstrap budget"));
    assert!(checker.type_work.is_none());
    for _ in 0..MAX_WORK + 1 {
        assert_eq!(
            checker.type_value(&literal()).unwrap(),
            Type::Int {
                bits: 32,
                signed: true
            }
        );
    }
}

#[test]
pub(crate) fn computed_types_bound_wide_nested_roots_and_materialized_types() {
    for (count, computed) in [(MAX_WORK + 1, true), (MAX_NODES + 1, false)] {
        let fields = (0..count)
            .map(|id| {
                let kind = if computed {
                    TypeKind::Computed(Box::new(literal()))
                } else {
                    TypeKind::Name("int32".into())
                };
                (
                    format!("n{id}"),
                    TypeExpr {
                        kind,
                        span: Span::new(5, 9),
                    },
                    false,
                )
            })
            .collect();
        let expr = Expr {
            kind: ExprKind::TypeValue(TypeExpr {
                kind: TypeKind::Record {
                    primary: None,
                    fields,
                },
                span: Span::new(2, 20),
            }),
            span: Span::new(2, 20),
        };
        let error = Checker::new().type_value(&expr).unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("computed type bootstrap budget"));
    }
}

#[test]
pub(crate) fn computed_types_blocks_construct_scoped_records_lists_and_nested_aliases() {
    for source in [
        "<T>:{element:<int32>;-><(element)[4]>};v<T>:[1,2]",
        "<T>:{<Local>:<{n<int32>}>;token:<Local>;->token};v<T>:{->n:7}",
        "<T>:{t:{-><int32>};-><(t)>};v<T>:7",
        "core:@\"core\";<T>:{t:core.int32;->t};v<T>:7",
        "t:<int32>;<T>:{t:<string>;->t};v<T>:\"yes\";n<(t)>:7",
        "<T>:{-><int32>;unused:<string>};v<T>:7",
    ] {
        crate::compile(source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
    }
}

#[test]
pub(crate) fn computed_types_blocks_reject_leaks_duplicate_emissions_and_wrong_results() {
    for (source, code) in [
        ("<T>:{inner:<int32>;->inner};v<(inner)>:1", "E201"),
        ("<T>:{<Inner>:<int32>;-><Inner>};v<Inner>:1", "E202"),
        ("<T>:{-><int32>;-><string>}", "E205"),
        ("<T>:{t:<int32>;t:<string>;->t}", "E203"),
        ("<T>:{<T>:<int32>;<T>:<string>;-><T>}", "E203"),
        ("<T>:{t:<int32>}", "E211"),
        ("x:1;<T>:{->x}", "E211"),
        ("<T>:{-><int32>};v<T>:true", "E207"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn computed_types_blocks_reject_effects_after_emissions_and_unsupported_control() {
    for source in [
        "d:@\"debug\";<T>:{d.print(1);-><int32>}",
        "d:@\"debug\";<T>:{(d.print(1));-><int32>}",
        "d:@\"debug\";fail:d.panic;<T>:{-><int32>;fail(\"no\")}",
        "d:@\"debug\";<T>:{t:d.print(1);-><int32>}",
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            "E219",
            "{source}"
        );
    }
    for source in [
        "<T>:{t:=<int32>;->t}",
        "<T>:{|true|-><int32>;|_|-><string>}",
        "f<int32>:(){->1};<T>:{->f()}",
        "d:@\"debug\";print<int32>:(){->1};<T>:{->print()}",
        "<T>:{->named:<int32>}",
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            "B001",
            "{source}"
        );
    }
}

#[test]
pub(crate) fn computed_types_blocks_share_work_limits_across_sibling_blocks() {
    let inner = (0..40)
        .map(|id| format!("t{id}:<int32>;"))
        .collect::<String>();
    let outer = (0..60)
        .map(|id| format!("t{id}:{{{inner}-><int32>}};"))
        .collect::<String>();
    let source = format!("<T>:{{{outer}-><int32>}}");
    let error = crate::compile(&source).unwrap_err().remove(0);
    assert_eq!(error.code, "B001");
    assert!(
        error.message.contains("computed type bootstrap budget"),
        "{error:?}"
    );
}

#[test]
pub(crate) fn computed_types_blocks_leave_no_runtime_statements_or_storage() {
    let program =
        crate::compile("<T>:{element:<int32>;row:{-><{n<int32>}>};-><(element)[4]>}").unwrap();
    assert!(program.body.stmts.is_empty());
    assert!(program.locals.is_empty());
    assert!(program.functions.is_empty());
}

#[test]
pub(crate) fn computed_types_documentation_derives_constructed_signatures() {
    let source = include_str!("../../../examples/computed-types.mwy");
    let (_, model) = crate::documentation::checked(source, true).unwrap();
    let model = model.unwrap();
    let counts = model
        .entries
        .iter()
        .find(|entry| entry.name == "Counts")
        .unwrap();
    assert_eq!(counts.signature, "int32[4]");
    let row = model
        .entries
        .iter()
        .find(|entry| entry.name == "Row")
        .unwrap();
    assert_eq!(row.signature, "{count<int32>;label<string>}");
}

#[test]
pub(crate) fn computed_integers_preserve_literal_widths_aliases_and_local_scope() {
    for source in [
        "<T>:{n:4;-><int32[n]>};v<T>:[1,2]",
        "<T>:{n<uint8>:4;copy:n;-><int32[copy]>};v<T>:[1]",
        "<T>:{n<uint64>:4294967296;copy:n;->copy<>};v<T>:4294967296",
    ] {
        crate::compile(source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
    }
    assert_eq!(
        crate::compile("<T>:{n<uint8>:256;-><int32>}").unwrap_err()[0].code,
        "E216"
    );
    assert_eq!(
        crate::compile("<T>:{n:4;-><int32[n]>};v:n").unwrap_err()[0].code,
        "E201"
    );
    let program = crate::compile("<T>:{n<uint8>:4;copy:n;-><int32[copy]>}").unwrap();
    assert!(program.locals.is_empty());
    assert!(program.body.stmts.is_empty());
}

#[test]
pub(crate) fn computed_integers_check_arithmetic_with_exact_width_and_signedness() {
    for source in [
        "<T>:{base:2;capacity:base*3+1;-><int32[capacity]>};v<T>:[1,2]",
        "<T>:{base<uint8>:254;capacity:base+1;-><int32[capacity]>}",
        "<T>:{n<uint8>:252;capacity:~n;-><int32[capacity]>}",
        "<T>:{n<int8>:-8;capacity:-n;-><int32[capacity]>}",
        "<T>:{n:((8/2)%3)|4;capacity:n^1;-><int32[capacity]>}",
    ] {
        crate::compile(source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
    }
    for (source, code) in [
        ("<T>:{n<uint8>:255;capacity:n+1;-><int32>}", "E107"),
        ("<T>:{n<int8>:-128;capacity:-n;-><int32>}", "E107"),
        ("<T>:{capacity:1/0;-><int32>}", "E107"),
        ("<T>:{a<uint8>:2;b<int32>:1;capacity:a+b;-><int32>}", "E213"),
        ("<T>:{capacity:-1;-><int32[capacity]>}", "E104"),
        ("|false|{<T>:{n<uint8>:255;capacity:n+1;-><int32>}}", "E107"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn computed_integers_reject_runtime_inputs_effects_and_unsupported_scalars() {
    for (source, code) in [
        (
            "f<int32>:(n<int32>){<T>:{capacity:n+1;-><int32[capacity]>};->1}",
            "E211",
        ),
        ("n:=3;<T>:{capacity:n+1;-><int32>}", "E211"),
        ("n:{scratch:=0;->3};<T>:{capacity:n+1;-><int32>}", "E211"),
        ("n:{scratch:=0;->3};<T>:{-><int32[n]>}", "E211"),
        (
            "d:@\"debug\";n:{d.print(1);->3};<T>:{capacity:n+1;-><int32>}",
            "E211",
        ),
        ("d:@\"debug\";<T>:{capacity:1+d.print(2);-><int32>}", "E219"),
        (
            "d:@\"debug\";<T>:{capacity<int32>:d.print(2);-><int32>}",
            "E219",
        ),
        ("f<int32>:(){->1};<T>:{capacity:1+f();-><int32>}", "B001"),
        ("<T>:{n<float32>:1.0;-><int32>}", "B001"),
        ("<T>:{n:=true;-><int32>}", "B001"),
        ("<T>:{n:1<2;-><int32>}", "B001"),
        ("<T>:{n:=3;-><int32>}", "B001"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn computed_integers_count_expression_work_within_the_type_root() {
    let binds = (0..700)
        .map(|id| format!("n{id}:1+2+3+4;"))
        .collect::<String>();
    let source = format!("<T>:{{{binds}-><int32>}}");
    let error = crate::compile(&source).unwrap_err().remove(0);
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("computed type bootstrap budget"));
}

#[test]
pub(crate) fn computed_integers_keep_documentation_widths_and_emit_no_runtime_scratch() {
    let source = include_str!("../../../examples/computed-types.mwy");
    let (_, model) = crate::documentation::checked(source, true).unwrap();
    let model = model.unwrap();
    for name in ["base", "capacity"] {
        let entry = model
            .entries
            .iter()
            .find(|entry| entry.name == name && entry.kind == crate::documentation::Kind::Binding)
            .unwrap();
        assert_eq!(entry.signature, "uint8");
        assert!(entry.checked);
        assert!(!entry.public);
    }
    let program =
        crate::compile("<T>:{base<uint8>:2;capacity:base*2;-><int32[capacity]>}").unwrap();
    assert!(program.locals.is_empty());
    assert!(program.functions.is_empty());
    assert!(program.body.stmts.is_empty());
}

#[test]
pub(crate) fn initializer_inputs_feed_required_bindings_extents_and_function_scopes() {
    for source in [
        "base<uint8>:2;capacity:base*2;alias:capacity;<T>:{n:alias;-><int32[n]>};v<T>:[1,2]",
        "capacity:4;<T>:{-><int32[capacity]>};v<T>:[1,2]",
        "capacity:4;f<int32>:(){<T>:{n:capacity;-><int32[n]>};values<T>:[1,2];->values[2]}",
        "capacity:4;<T>:{capacity:capacity+1;-><int32[capacity]>}",
        "base:2;capacity:base+1;<T>:{base:99;-><int32[capacity]>}",
    ] {
        crate::compile(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
    }
    let source = "capacity:4;f<int32>:(){->capacity}";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "B001");
}

#[test]
pub(crate) fn initializer_inputs_report_hidden_failures_at_the_original_expression() {
    for expression in ["255+1", "1/0", "(255+1)*0"] {
        let source =
            format!("|false|{{bad<uint8>:{expression};alias:bad;<T>:{{n:alias;-><int32>}}}}");
        let error = crate::compile(&source).unwrap_err().remove(0);
        assert_eq!(error.code, "E107", "{error:?}");
        let start = source.find(expression).unwrap();
        assert!(error.span.start >= start && error.span.end <= start + expression.len());
    }
}

#[test]
pub(crate) fn initializer_inputs_charge_transitive_work_on_cached_reads() {
    let binds = (1..14)
        .map(|id| format!("v{id}:v{}+v{};", id - 1, id - 1))
        .collect::<String>();
    let source = format!("v0:1;{binds}<T>:{{n:v13;-><int32>}}");
    let error = crate::compile(&source).unwrap_err().remove(0);
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("computed type bootstrap budget"));
    let roots = (0..1500)
        .map(|id| format!("<T{id}>:{{n:capacity;-><int32[n]>}};"))
        .collect::<String>();
    crate::compile(&format!("capacity:4;{roots}")).unwrap();
}
