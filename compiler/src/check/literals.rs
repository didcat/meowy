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
                "core.literal expects one numeric syntax argument",
                expr.span,
            ));
        };
        let (leaf, negative) = match &arg.kind {
            ExprKind::Unary { op, value } if op == "-" => (value.as_ref(), true),
            _ => (arg, false),
        };
        let mut leaf = match &leaf.kind {
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
                    "core.literal requires a numeric token, optionally preceded by `-`",
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
