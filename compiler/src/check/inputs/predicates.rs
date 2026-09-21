use crate::check::{
    Checker,
    inputs::{Input, Sources},
    type_values,
};
use crate::hir::{Expr, ExprKind, Type};

impl Checker {
    pub(crate) fn boolean_input(&mut self, expr: &Expr, ty: &Type) -> Option<Input<bool>> {
        if *ty != Type::Bool {
            return None;
        }
        self.predicate_expr(expr, 0, &mut 0, &Sources::default())
    }

    pub(crate) fn predicate_expr(
        &mut self,
        expr: &Expr,
        depth: usize,
        count: &mut usize,
        locals: &Sources,
    ) -> Option<Input<bool>> {
        *count += 1;
        if *count > type_values::MAX_WORK
            || depth >= type_values::MAX_DEPTH
            || !self.flow.spend(1)
            || !(expr.ty == Type::Bool
                || expr.ty == Type::Never && matches!(expr.kind, ExprKind::Block(_)))
        {
            return None;
        }
        let mut input = Input {
            derived: false,
            work: 1,
            error: None,
            value: None,
        };
        match &expr.kind {
            ExprKind::Bool(value) => input.value = Some(*value),
            ExprKind::Local(id) => {
                let source = locals
                    .booleans
                    .get(id)
                    .or_else(|| self.bool_inputs.get(id))
                    .or_else(|| self.module_boolean(*id))?;
                input.add(source);
                input.value = source.value;
            }
            ExprKind::Primary(value) => {
                let ExprKind::Local(id) = value.kind else {
                    return None;
                };
                let source = self.module_boolean(id)?;
                input.add(source);
                input.value = source.value;
            }
            ExprKind::Unary { op, value } if op == "!" => {
                let source = self.predicate_expr(value, depth + 1, count, locals)?;
                input.add(&source);
                input.value = source.value.map(|value| !value);
            }
            ExprKind::Binary { op, left, right } if matches!(op.as_str(), "&&" | "||") => {
                let left = self.predicate_expr(left, depth + 1, count, locals)?;
                input.add(&left);
                input.value = left.value;
                if input.error.is_some() || input.value == Some(op == "||") {
                    return Some(input);
                }
                let right = self.predicate_expr(right, depth + 1, count, locals)?;
                input.add(&right);
                input.value = right.value;
            }
            ExprKind::Binary { op, left, right }
                if matches!(op.as_str(), "==" | "!=")
                    && left.ty == Type::Bool
                    && right.ty == Type::Bool =>
            {
                let a = self.predicate_expr(left, depth + 1, count, locals)?;
                input.add(&a);
                if input.error.is_some() {
                    return Some(input);
                }
                let b = self.predicate_expr(right, depth + 1, count, locals)?;
                input.add(&b);
                if input.error.is_none() {
                    let (a, b) = (a.value?, b.value?);
                    input.value = Some(if op == "==" { a == b } else { a != b });
                }
            }
            ExprKind::Binary { op, left, right }
                if matches!(op.as_str(), "==" | "!=" | "<" | "<=" | ">" | ">=")
                    && matches!(left.ty, Type::Int { .. })
                    && left.ty == right.ty =>
            {
                let a = self.input_expr(left, depth + 1, count, locals)?;
                input.add(&a);
                if input.error.is_some() {
                    return Some(input);
                }
                let b = self.input_expr(right, depth + 1, count, locals)?;
                input.add(&b);
                if input.error.is_none() {
                    let (a, b) = (a.value?, b.value?);
                    input.value = Some(match op.as_str() {
                        "==" => a == b,
                        "!=" => a != b,
                        "<" => a < b,
                        "<=" => a <= b,
                        ">" => a > b,
                        ">=" => a >= b,
                        _ => unreachable!(),
                    });
                }
            }
            ExprKind::Field { .. } => {
                let (id, path) = self.record_path(expr)?;
                let source = self.boolean_field_input(id, &path, locals)?;
                input.add(&source);
                input.value = source.value;
            }
            ExprKind::Block(block) => {
                let source = self.boolean_block(block, depth + 1, count, locals)?;
                input.add(&source);
                if expr.ty == Type::Never {
                    return input.error.is_some().then_some(input);
                }
                input.value = source.value;
            }
            _ => return None,
        }
        Some(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Span;

    #[test]
    pub(crate) fn literal_conditions_bound_visited_nodes_and_active_depth() {
        let expr = Expr {
            kind: ExprKind::Bool(true),
            ty: Type::Bool,
            span: Span { start: 0, end: 4 },
        };
        let mut checker = Checker::new();
        let mut count = type_values::MAX_WORK - 1;
        assert_eq!(
            checker
                .predicate_expr(&expr, 0, &mut count, &Sources::default())
                .map(|input| (input.value, input.work)),
            Some((Some(true), 1))
        );
        assert!(
            checker
                .predicate_expr(&expr, 0, &mut count, &Sources::default())
                .is_none()
        );
        assert_eq!(
            checker
                .predicate_expr(
                    &expr,
                    type_values::MAX_DEPTH - 1,
                    &mut 0,
                    &Sources::default()
                )
                .map(|input| (input.value, input.work)),
            Some((Some(true), 1))
        );
        assert!(
            checker
                .predicate_expr(&expr, type_values::MAX_DEPTH, &mut 0, &Sources::default())
                .is_none()
        );
    }
}
