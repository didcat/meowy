pub(crate) mod effect;
pub(crate) mod pure;
pub(crate) mod source;
pub(crate) mod types;

use crate::ast::{self, Span};
use crate::check::{Checker, Result};
use crate::diagnostic::Diagnostic;
use crate::flow::{FALSE, Guard};
use crate::hir::{self, Type};

pub(crate) use types::{Fit, MAX_CONTEXTS, MAX_SCALAR_NODES, Scalar};

impl Checker {
    pub(crate) fn list_union(
        &mut self,
        values: &[ast::Expr],
        contexts: &[&Type],
        span: Span,
    ) -> Result<hir::Expr> {
        if contexts.len() > MAX_CONTEXTS {
            return Err(Diagnostic::unsupported(
                "list candidate budget exhausted",
                span,
            ));
        }
        let mut choices: Vec<_> = contexts
            .iter()
            .copied()
            .filter(|ty| matches!(ty, Type::List { capacity, .. } if *capacity >= values.len()))
            .collect();
        if choices.is_empty() {
            return Err(Self::error(
                "E103",
                format!(
                    "{} elements exceed every expected list capacity",
                    values.len()
                ),
                span,
            ));
        }
        if choices.len() == 1 {
            return self.list_literal(values, Some(choices[0]), span);
        }
        let mut viable = Vec::new();
        for choice in choices {
            let Type::List { element, .. } = choice else {
                unreachable!()
            };
            let mut fit = Fit::Yes;
            for value in values {
                fit = fit.and(self.list_probe(value, element, FALSE)?);
            }
            if fit != Fit::No {
                viable.push(choice);
            }
        }
        choices = viable;
        if choices.is_empty() {
            return Err(Self::error(
                "E207",
                "list literal fits no expected list type",
                span,
            ));
        }
        if choices.len() == 1 {
            return self.list_literal(values, Some(choices[0]), span);
        }
        let mut items = vec![None; values.len()];
        let mut points = vec![None; values.len()];
        let mut deferred = Vec::new();
        for (index, value) in values.iter().enumerate() {
            if choices.len() > 1 && self.list_deferred(value)? {
                choices = self.list_filter(value, choices, self.reach)?;
                if choices.len() > 1 {
                    deferred.push((index, self.reach));
                    continue;
                }
            }
            let elements: Vec<_> = choices
                .iter()
                .map(|ty| {
                    let Type::List { element, .. } = ty else {
                        unreachable!()
                    };
                    element.as_ref()
                })
                .collect();
            let common = elements.iter().all(|ty| *ty == elements[0]);
            let item = if !common
                && let Some((point, item)) = self.list_effect_block(
                    value,
                    &mut choices,
                    !deferred.is_empty() || index + 1 < values.len(),
                )? {
                points[index] = Some(point);
                item
            } else if common {
                let (point, value) = self.expr_point(value, Some(elements[0]))?;
                points[index] = Some(point);
                value
            } else if self.list_independent(value) {
                let (point, value) = self.expr_point(value, None)?;
                points[index] = Some(point);
                value
            } else {
                return Err(Diagnostic::unsupported(
                    "list element needs a concrete contextual type before its effects can be checked",
                    value.span,
                ));
            };
            let mut matching = Vec::new();
            for ty in choices {
                let Type::List { element, .. } = ty else {
                    unreachable!()
                };
                crate::borrow_contract::type_weight(&item.ty, &mut self.flow, value.span)?;
                if Self::list_assigns(element, &item.ty) {
                    matching.push(ty);
                }
            }
            choices = matching;
            if choices.is_empty() {
                return Err(Self::error(
                    "E207",
                    "list element fits no expected list type",
                    value.span,
                ));
            }
            items[index] = Some(item);
        }
        for (index, reach) in &deferred {
            if choices.len() == 1 {
                break;
            }
            choices = self.list_filter(&values[*index], choices, *reach)?;
        }
        if choices.len() != 1 {
            for choice in &choices {
                let Type::List { element, .. } = choice else {
                    unreachable!()
                };
                for (index, reach) in &deferred {
                    if self.list_probe(&values[*index], element, *reach)? == Fit::Unknown {
                        return Err(Diagnostic::unsupported(
                            "list candidates need unresolved nested contextual inference",
                            values[*index].span,
                        ));
                    }
                }
            }
            return Err(Self::error(
                "E207",
                "list literal fits multiple expected list types",
                span,
            ));
        }
        let list = choices[0].clone();
        let Type::List { element, .. } = &list else {
            unreachable!()
        };
        for (index, reach) in deferred {
            let after = std::mem::replace(&mut self.reach, reach);
            let result = self.expr_point(&values[index], Some(element));
            self.reach = after;
            let (point, value) = result?;
            points[index] = Some(point);
            items[index] = Some(value);
        }
        let values = items
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                Self::expected_value(
                    value.expect("checked list element"),
                    element,
                    values[index].span,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let ty = if values.iter().any(|value| value.ty == Type::Never) {
            Type::Never
        } else {
            list.clone()
        };
        self.list_sequence(
            self.point.expect("union list literal"),
            points,
            ty != Type::Never,
            span,
        )?;
        Ok(hir::Expr {
            kind: hir::ExprKind::List { values, list },
            ty,
            span,
        })
    }

    pub(crate) fn list_filter<'a>(
        &mut self,
        value: &ast::Expr,
        choices: Vec<&'a Type>,
        reach: Guard,
    ) -> Result<Vec<&'a Type>> {
        let mut matching = Vec::new();
        for ty in choices {
            let Type::List { element, .. } = ty else {
                unreachable!()
            };
            if self.list_probe(value, element, reach)? != Fit::No {
                matching.push(ty);
            }
        }
        if matching.is_empty() {
            return Err(Self::error(
                "E207",
                "list element fits no expected list type",
                value.span,
            ));
        }
        Ok(matching)
    }
}

#[cfg(test)]
mod tests;
