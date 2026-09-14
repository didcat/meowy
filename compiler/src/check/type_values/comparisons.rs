mod blocks;
mod equality;
mod types;

use crate::ast;
use crate::check::{Checker, Constant, Result, Value};
use crate::hir::Type;

impl Checker {
    pub(crate) fn integer_comparison_form(
        &mut self,
        op: &str,
        left: &ast::Expr,
        right: &ast::Expr,
        depth: usize,
        count: &mut usize,
    ) -> Result<Option<Type>> {
        let a = self.required_hint(left);
        let b = self.required_hint(right);
        if matches!(op, "==" | "!=")
            && matches!(
                (&a, &b),
                (Some(Type::Record { .. }), Some(Type::Record { .. }))
            )
        {
            return Ok(None);
        }
        let context = a.or(b).map(|ty| Self::primary_type(&ty));
        if context
            .as_ref()
            .is_some_and(|ty| !matches!(ty, Type::Int { .. }))
        {
            return Ok(None);
        }
        let a = self.integer_form(left, context.as_ref(), depth, count)?;
        let b = self.integer_form(right, Some(&a), depth, count)?;
        Self::integer_operands(op, &a, &b, ast::Span::new(left.span.start, right.span.end))?;
        Ok(Some(a))
    }

    pub(crate) fn required_comparison(
        &mut self,
        op: &str,
        left: &ast::Expr,
        right: &ast::Expr,
    ) -> Result<Option<bool>> {
        if self.type_comparison_form(
            op,
            left,
            right,
            self.type_work.as_ref().unwrap().depth,
            &mut 0,
        )? {
            let left = self.type_value(left)?;
            let right = self.type_value(right)?;
            return Ok(Some(if op == "==" {
                left == right
            } else {
                left != right
            }));
        }
        if self.integer_blocks(left)? || self.integer_blocks(right)? {
            let context = self.block_comparison_form(
                op,
                left,
                right,
                self.type_work.as_ref().unwrap().depth,
                &mut 0,
            )?;
            if matches!(op, "==" | "!=") {
                return self
                    .required_block_equality(op, left, right, context.as_ref())
                    .map(Some);
            }
            let Value::Static {
                value: Constant::Int(a),
                ty,
            } = self.integer_arithmetic(left, context.as_ref())?
            else {
                unreachable!()
            };
            let Value::Static {
                value: Constant::Int(b),
                ..
            } = self.integer_arithmetic(right, Some(&ty))?
            else {
                unreachable!()
            };
            return Ok(Some(Self::compare_integers(op, a, b)));
        }
        let Some(ty) = self.integer_comparison_form(
            op,
            left,
            right,
            self.type_work.as_ref().unwrap().depth,
            &mut 0,
        )?
        else {
            return Ok(None);
        };
        let left = self.required_integer(left, &ty)?;
        let right = self.required_integer(right, &ty)?;
        Ok(Some(Self::compare_integers(op, left, right)))
    }

    pub(crate) fn compare_integers(op: &str, left: i128, right: i128) -> bool {
        match op {
            "==" => left == right,
            "!=" => left != right,
            "<" => left < right,
            ">" => left > right,
            "<=" => left <= right,
            ">=" => left >= right,
            _ => unreachable!(),
        }
    }
}
