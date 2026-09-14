use crate::ast::{Expr, ExprKind, Span};
use crate::check::{Checker, Result, Value};

impl Checker {
    pub(crate) fn type_comparison_form(
        &mut self,
        op: &str,
        left: &Expr,
        right: &Expr,
        depth: usize,
        count: &mut usize,
    ) -> Result<bool> {
        let a = self.type_operand_form(left, depth, count)?;
        let b = self.type_operand_form(right, depth, count)?;
        if !a && !b {
            return Ok(false);
        }
        if !a || !b || !matches!(op, "==" | "!=") {
            return Err(Self::error(
                "E222",
                "type values support only equality with another type value",
                Span::new(left.span.start, right.span.end),
            ));
        }
        Ok(true)
    }

    pub(crate) fn type_operand_form(
        &mut self,
        expr: &Expr,
        depth: usize,
        count: &mut usize,
    ) -> Result<bool> {
        self.form_work(expr, depth, count)?;
        match &expr.kind {
            ExprKind::TypeValue(_) | ExprKind::TypeQuery(_) => Ok(true),
            ExprKind::Group(value) => self.type_operand_form(value, depth + 1, count),
            ExprKind::Name(_) | ExprKind::Field { .. } => {
                let mut base = expr;
                let mut depth = depth;
                while let ExprKind::Field { value, .. } | ExprKind::Group(value) = &base.kind {
                    base = value;
                    depth += 1;
                    self.form_work(base, depth, count)?;
                }
                if !matches!(base.kind, ExprKind::Name(_) | ExprKind::Import(_)) {
                    return Ok(false);
                }
                let saved = std::mem::replace(&mut self.required, true);
                let symbol = self.symbol(expr);
                self.required = saved;
                Ok(matches!(
                    symbol?,
                    Some(Value::Type(_) | Value::Foundation(crate::foundation::Item::Type(_)))
                ))
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub(crate) fn type_equality_uses_normalized_identity_without_runtime_storage() {
        for (left, right, same) in [
            ("<int32>", "<int32>", true),
            ("<int32>", "<uint32>", false),
            ("<int32><null><never>", "<null><int32><int32>", true),
            ("<{a<int32>;b<boolean>}>", "<{b<boolean>;a<int32>}>", true),
            ("<{a<int32>}>", "<{a<int32>:=}>", false),
            ("<int32[2]>", "<int32[1+1]>", true),
            ("<int32[2]>", "<int32[3]>", false),
            ("<&int32>", "<&!int32>", false),
        ] {
            for (op, expected) in [("==", same), ("!=", !same)] {
                let source =
                    format!("<T>:{{flag:{left} {op} {right};|flag|-><int32>;|!flag|-><boolean>}}");
                let program = crate::compile(&source).unwrap();
                assert!(program.locals.is_empty());
                assert!(program.body.stmts.is_empty());
                let value = if expected { "7" } else { "true" };
                crate::compile(&format!("{source};v<T>:{value}")).unwrap();
            }
        }
        crate::compile(
            "kind<Type>:<int32>;<T>:{copy:kind;flag:(copy)==7<>;|flag|-><int32>};v<T>:7",
        )
        .unwrap();
    }

    #[test]
    pub(crate) fn type_equality_skips_construction_but_checks_operand_kinds() {
        crate::compile("<T>:{flag:false&&(<int32[1/0]> == <Missing>);other:true||(<(missing())> != <int32>);-><int32>}").unwrap();
        for (expr, code) in [
            ("<int32> == true", "E222"),
            ("1!=<int32>", "E222"),
            ("false&&(<int32> == false)", "E222"),
            ("<int32> < <int32>", "E222"),
            ("<int32> == missing", "E201"),
            ("<int32> == <Missing>", "E202"),
            ("<int32> == <int32[1/0]>", "E107"),
            ("<int32> == ({-><int32>})", "E222"),
        ] {
            let source = format!("<T>:{{flag:{expr};-><int32>}}");
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{source}: {error:?}");
        }
    }
}
