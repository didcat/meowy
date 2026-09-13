use crate::ast::{self, ExprKind};
use crate::check::{Checker, Result, Value};
use crate::hir::Type;

impl Checker {
    pub(crate) fn scalar_block(&mut self, expr: &ast::Expr, ty: &Type) -> Result<Value> {
        self.type_work.as_mut().unwrap().enter(expr.span)?;
        let result = match &expr.kind {
            ExprKind::Group(value) => self.scalar_block(value, ty),
            ExprKind::Block(block) => self.required_block(block, Some(ty)),
            _ => unreachable!(),
        };
        self.type_work.as_mut().unwrap().depth -= 1;
        result
    }

    pub(crate) fn scalar_emission(&mut self, expr: &ast::Expr, ty: &Type) -> Result<Value> {
        let mut form = expr;
        while let ExprKind::Group(value) = &form.kind {
            form = value;
        }
        if matches!(form.kind, ExprKind::Block(_)) {
            return self.scalar_block(expr, ty);
        }
        if self.boolean_scalar(expr) {
            let value = self.type_boolean(expr, None)?;
            if *ty != Type::Bool {
                return Err(Self::error(
                    "E207",
                    format!("expected {ty:?}, found {:?}", Type::Bool),
                    expr.span,
                ));
            }
            return Ok(value);
        }
        self.scalar_input(expr)?;
        self.integer_result(expr, Some(ty))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::Constant;

    pub(crate) fn expression(source: &str) -> ast::Expr {
        let parsed = crate::parser::parse(&format!("value:{source}")).unwrap();
        let ast::StmtKind::Bind { value, .. } = &parsed.stmts[0].kind else {
            panic!("binding")
        };
        value.clone()
    }

    #[test]
    pub(crate) fn required_scalar_blocks_keep_values_widths_and_no_runtime_storage() {
        let mut checker = Checker::new();
        checker.type_work = Some(super::super::Work::default());
        let expr = expression("{->{->false};unused:7}");
        assert!(matches!(
            checker.scalar_block(&expr, &Type::Bool).unwrap(),
            Value::Static {
                value: Constant::Bool(false),
                ty: Type::Bool
            }
        ));
        let expr = expression("{n<uint8>:{->2};->n+2}");
        assert!(matches!(
            checker
                .scalar_block(
                    &expr,
                    &Type::Int {
                        bits: 8,
                        signed: false
                    }
                )
                .unwrap(),
            Value::Static {
                value: Constant::Int(4),
                ty: Type::Int {
                    bits: 8,
                    signed: false
                }
            }
        ));
        let program = crate::compile(
            "<T>:{flag<boolean>:{->true};n<uint8>:({|flag|->{->4};|!flag|->2});-><int32[n]>}",
        )
        .unwrap();
        assert!(program.body.stmts.is_empty());
        assert!(program.locals.is_empty());
        assert!(program.functions.is_empty());
    }

    #[test]
    pub(crate) fn required_scalar_blocks_preserve_primary_kind_scope_and_tail_errors() {
        for (body, code) in [
            ("n<uint8>:{->true}", "E207"),
            ("n<boolean>:{->1}", "E207"),
            ("n<uint8>:{->256}", "E216"),
            ("n<uint8>:{v<uint8>:255;->v+1}", "E107"),
            ("n<uint8>:{->1;->2}", "E205"),
            ("n<boolean>:{|false|->true}", "E204"),
            ("n<uint8>:{->1;unused:1/0}", "E107"),
            ("n<uint8>:{private<uint8>:4;->private};copy:private", "E201"),
            ("n:{->4}", "B001"),
            ("n<float32>:{->1.0}", "B001"),
            ("n<uint8>:{v:=1;->v}", "B001"),
            ("n<uint8>:{|true|{->4}}", "B001"),
        ] {
            let source = format!("<T>:{{{body};-><int32>}}");
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{source}: {error:?}");
        }
    }
}
