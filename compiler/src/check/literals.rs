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
        if !matches!(self.symbol(callee)?, Some(Value::Foundation(Item::Literal))) {
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
