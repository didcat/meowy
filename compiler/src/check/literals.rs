use super::{Checker, Result, Value};
use crate::ast::{Expr, ExprKind};
use crate::foundation::Item;

impl Checker {
    pub(crate) fn expanded_expression(&mut self, expr: &Expr) -> Result<Option<Box<Expr>>> {
        if let Some(value) = self.literal_expression(expr)? {
            return Ok(Some(value));
        }
        self.bits_expression(expr).map(|value| value.map(Box::new))
    }

    pub(crate) fn literal_expression(&mut self, expr: &Expr) -> Result<Option<Box<Expr>>> {
        let ExprKind::Call { callee, args } = &expr.kind else {
            return Ok(None);
        };
        if !matches!(
            self.hint_symbol(callee),
            Some(Value::Foundation(Item::Literal))
        ) {
            return Ok(None);
        }
        let [arg] = args.as_slice() else {
            return Err(Self::error(
                "E212",
                "core.literal expects one literal syntax argument",
                expr.span,
            ));
        };
        let (leaf, negative) = match &arg.kind {
            ExprKind::Unary { op, value } if op == "-" => (value.as_ref(), true),
            _ => (arg, false),
        };
        let mut leaf = match &leaf.kind {
            ExprKind::Name(name)
                if !negative && matches!(name.as_str(), "true" | "false" | "null") =>
            {
                Expr {
                    kind: ExprKind::Field {
                        value: Box::new(Expr {
                            kind: ExprKind::Import("core".into()),
                            span: leaf.span,
                        }),
                        name: name.clone(),
                    },
                    span: leaf.span,
                }
            }
            ExprKind::Int(number) | ExprKind::Float(number) => {
                let mut number = number.clone();
                number.literal = true;
                Expr {
                    kind: if matches!(leaf.kind, ExprKind::Int(_)) {
                        ExprKind::Int(number)
                    } else {
                        ExprKind::Float(number)
                    },
                    span: leaf.span,
                }
            }
            _ => {
                return Err(Self::error(
                    "E207",
                    "core.literal requires `true`, `false`, `null`, or a numeric token optionally preceded by `-`",
                    arg.span,
                ));
            }
        };
        if negative {
            leaf = Expr {
                kind: ExprKind::Unary {
                    op: "-".into(),
                    value: Box::new(leaf),
                },
                span: expr.span,
            };
        } else {
            leaf.span = expr.span;
        }
        Ok(Some(Box::new(leaf)))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub(crate) fn literal_identity_preserves_syntax_context_and_aliases() {
        for source in [
            "c:@\"core\";1:7;x<uint8>:c.literal(1);y<float32>:c.literal(1.5)",
            "c:@\"core\";lit:c.literal;128:7;x<int8>:lit(-128)",
            "literal:@\"core\".literal;0x1:true;x:literal(0x1)",
            "1:@\"core\".literal;x:1(1)",
            "c:@\"core\";{c:{->literal:7};x:c.literal};x:c.literal(01)",
        ] {
            crate::compile(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
        }
        for (arg, code) in [
            ("", "E212"),
            ("1,2", "E212"),
            ("true", "E207"),
            ("missing", "E207"),
            ("1+2", "E207"),
            ("(1)", "E207"),
            ("-256", "E216"),
        ] {
            let source = format!("x<int8>:@\"core\".literal({arg})");
            let errors = crate::compile(&source).unwrap_err();
            assert_eq!(errors[0].code, code, "{source}: {errors:?}");
        }
    }
}

#[cfg(test)]
mod required {
    #[test]
    pub(crate) fn literal_syntax_preserves_required_values_and_queries() {
        for source in [
            r#"c:@"core";1:true;<T>:{n:c.literal(1);-><int32[n]>};v<T>:[7]"#,
            r#"lit:@"core".literal;<T>:{128:true;n<int8>:lit(-128);flag:n<lit(0);->n<>};v<T>:-7"#,
            r#"lit:@"core".literal;<T>:{1:2;n:({->lit(1)})+lit(2);-><int32[n]>};v<T>:[0,0,0]"#,
            r#"1:true;<T>:@"core".literal(1)<>;v<T>:7"#,
            r#"lit:@"core".literal;<T>:{1:true;n:lit(1);copy:n;->copy<>};v<T>:7"#,
        ] {
            crate::compile(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
        }
    }
}

#[cfg(test)]
mod lists {
    #[test]
    pub(crate) fn literal_calls_keep_contextual_list_and_extent_rules() {
        for source in [
            r#"lit:@"core".literal;1:true;x<uint8>:3;v:[lit(1),x];n<uint8>:v[01]"#,
            r#"lit:@"core".literal;1:true;v<uint8[01]><uint16[01]>:[lit(256)]"#,
            r#"lit:@"core".literal;128:7;v<int8[01]><uint8[01]>:[lit(-128)]"#,
            r#"lit:@"core".literal;1:true;v<int32[lit(1)]>:[7]"#,
            r#"lit:@"core".literal;1:true;v<uint8[01]><uint16[01]>:[lit(255)+lit(1)]"#,
            r#"lit:@"core".literal;1:true;v<float32[01]><int32[01]>:[lit(1.5)]"#,
        ] {
            crate::compile(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
        }
    }
}

#[cfg(test)]
mod rejected {
    #[test]
    pub(crate) fn required_boolean_forms_check_literal_kinds_even_when_skipped() {
        for expression in ["!literal(1)", "false&&literal(1)", "true||literal(1.5)"] {
            let source = format!(r#"literal:@"core".literal;<T>:{{flag:{expression};->flag<>}}"#);
            let errors = crate::compile(&source).unwrap_err();
            assert_eq!(errors[0].code, "E222", "{source}: {errors:?}");
        }
    }
}

#[cfg(test)]
mod lookup {
    #[test]
    pub(crate) fn literal_probes_defer_unresolved_calls_to_the_original_context() {
        for (source, code) in [
            (r#"x:missing(1)"#, "E201"),
            (r#"x<int32[missing(1)]>:[]"#, "B001"),
            (r#"c:@"core";x<int32[c.literal(missing)]>:[]"#, "E207"),
        ] {
            let errors = crate::compile(source).unwrap_err();
            assert_eq!(errors[0].code, code, "{source}: {errors:?}");
        }
    }
}

#[cfg(test)]
mod booleans {
    #[test]
    pub(crate) fn boolean_literal_escape_bypasses_shadowing_and_preserves_callable_identity() {
        for source in [
            r#"true:"shadowed";false:7;x<boolean>:@"core".literal(true);y<boolean>:@"core".literal(false)"#,
            r#"core:@"core";literal:core.literal;true:"shadowed";x<boolean>:literal(true)"#,
            r#"true:@"core".literal;false:"shadowed";x<boolean>:true(false)"#,
            r#"literal<int32>:(value<int32>){->value};true:7;x<int32>:literal(true)"#,
        ] {
            crate::compile(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
        }
    }

    #[test]
    pub(crate) fn boolean_literal_escape_keeps_required_and_contextual_list_values() {
        for source in [
            r#"lit:@"core".literal;true:"shadowed";false:7;<T>:{flag:lit(true);other:!lit(false);both:lit(true)&&lit(false);->flag<>};x<T>:lit(false)"#,
            r#"lit:@"core".literal;true:7;false:"shadowed";v:[lit(true),lit(false)]"#,
            r#"lit:@"core".literal;true:7;v<boolean[1]><int32[1]>:[lit(true)]"#,
            r#"lit:@"core".literal;false:7;v<boolean[1]><float32[1]>:[lit(false)]"#,
            r#"lit:@"core".literal;<T>:lit(true)<>;x<T>:lit(false)"#,
        ] {
            crate::compile(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
        }
    }

    #[test]
    pub(crate) fn boolean_literal_escape_rejects_expressions_negation_and_wrong_types() {
        for arg in [
            "(true)",
            "!false",
            "-true",
            "-false",
            "true&&false",
            "flag",
            "True",
            "\"true\"",
        ] {
            let source = format!(r#"flag:true;x:@"core".literal({arg})"#);
            let errors = crate::compile(&source).unwrap_err();
            assert_eq!(errors[0].code, "E207", "{source}: {errors:?}");
        }
        for source in [
            r#"x<int32>:@"core".literal(true)"#,
            r#"x<int32>:@"core".literal(false)"#,
        ] {
            assert_eq!(crate::compile(source).unwrap_err()[0].code, "E207");
        }
        for args in ["", "true,false"] {
            assert_eq!(
                crate::compile(&format!(r#"x:@"core".literal({args})"#)).unwrap_err()[0].code,
                "E212"
            );
        }
        assert_eq!(
            crate::compile(r#"x<int32[@"core".literal(true)]>:[]"#).unwrap_err()[0].code,
            "E104"
        );
    }

    #[test]
    pub(crate) fn boolean_literal_escape_charges_literal_work_without_argument_reads() {
        for (source, steps) in [
            (r#"@"core".literal(true)"#, 1),
            (r#"!@"core".literal(false)"#, 2),
            (r#"@"core".literal(true)||@"core".literal(false)"#, 2),
            (r#"@"core".literal(true)&&@"core".literal(false)"#, 3),
        ] {
            let mut checker = crate::check::Checker::new();
            let block = crate::parser::parse(&format!("flag:{source}")).unwrap();
            let crate::ast::StmtKind::Bind { value, .. } = &block.stmts[0].kind else {
                panic!()
            };
            checker
                .required_root(value.span, |checker| {
                    checker.type_boolean(value, None)?;
                    assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, steps);
                    Ok(())
                })
                .unwrap();
            assert!(checker.body_inputs.is_empty());
        }
    }
}

#[cfg(test)]
mod nulls {
    #[test]
    pub(crate) fn null_literal_escape_bypasses_shadowing_and_preserves_callable_identity() {
        for source in [
            r#"x<null>:@"core".literal(null)"#,
            r#"null:"shadowed";x<null>:@"core".literal(null)"#,
            r#"core:@"core";literal:core.literal;null:=7;x<null>:literal(null)"#,
            r#"null:@"core".literal;x<null>:null(null)"#,
            r#"literal<int32>:(value<int32>){->value};null:7;x<int32>:literal(null)"#,
            r#"f<null>:(null<int32>){->@"core".literal(null)};x<null>:f(7)"#,
        ] {
            crate::compile(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
        }
    }

    #[test]
    pub(crate) fn null_literal_escape_keeps_queries_nullable_values_and_list_contexts() {
        for source in [
            r#"null:7;<T>:@"core".literal(null)<>;x<T>:@"core".literal(null)"#,
            r#"lit:@"core".literal;null:7;<T>:{->lit(null)<>};x<T>:lit(null)"#,
            r#"f<null>:(null<int32>){<T>:@"core".literal(null)<>;x<T>:@"core".literal(null);->x}"#,
            r#"lit:@"core".literal;null:7;x<int32><null>:lit(null)"#,
            r#"lit:@"core".literal;null:7;v:[lit(null),lit(null)];x<null>:v[1]"#,
            r#"lit:@"core".literal;null:7;v<null[1]><int32[1]>:[lit(null)]"#,
            r#"lit:@"core".literal;null:7;<Element>:<int32><null>;v<Element[2]>:[lit(null),7]"#,
        ] {
            crate::compile(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
        }
    }

    #[test]
    pub(crate) fn null_literal_escape_keeps_syntax_type_and_arity_boundaries() {
        for arg in [
            "(null)",
            "-null",
            "!null",
            "null==null",
            "Null",
            "never",
            "@\"core\".null",
            "\"null\"",
            "[null]",
            "{->null}",
            "<null>",
        ] {
            let source = format!(r#"x:@"core".literal({arg})"#);
            let errors = crate::compile(&source).unwrap_err();
            assert_eq!(errors[0].code, "E207", "{source}: {errors:?}");
        }
        for (source, code) in [
            (r#"x<int32>:@"core".literal(null)"#, "E207"),
            (r#"x<boolean>:@"core".literal(null)"#, "E207"),
            (r#"x<int32[@"core".literal(null)]>:[]"#, "E104"),
            (r#"x:@"core".literal(null,null)"#, "E212"),
            (
                r#"<T>:{flag:false&&@"core".literal(null);->flag<>}"#,
                "E222",
            ),
        ] {
            let errors = crate::compile(source).unwrap_err();
            assert_eq!(errors[0].code, code, "{source}: {errors:?}");
        }
    }
}
