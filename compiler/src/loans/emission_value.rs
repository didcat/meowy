use super::{Expr, ExprKind, Graph, LocalId, Result, Type};
use std::collections::BTreeMap;

#[derive(Clone)]
pub(crate) enum Value {
    Unknown,
    Known(bool),
    Local(LocalId),
    Not(Box<Value>),
    Binary(Op, Box<Value>, Box<Value>),
}

#[derive(Clone, Copy)]
pub(crate) enum Op {
    And,
    Or,
    Equal,
    Different,
}

impl Value {
    pub(crate) fn negated(self) -> Self {
        match self {
            Self::Unknown => Self::Unknown,
            Self::Known(value) => Self::Known(!value),
            value => Self::Not(Box::new(value)),
        }
    }

    pub(crate) fn weight(&self) -> usize {
        match self {
            Self::Not(value) => value.weight() + 1,
            Self::Binary(_, left, right) => left.weight() + right.weight() + 1,
            _ => 1,
        }
    }

    pub(crate) fn get(&self, values: &BTreeMap<LocalId, bool>) -> Option<bool> {
        match self {
            Self::Unknown => None,
            Self::Known(value) => Some(*value),
            Self::Local(id) => values.get(id).copied(),
            Self::Not(value) => value.get(values).map(|value| !value),
            Self::Binary(op, left, right) => match (op, left.get(values), right.get(values)) {
                (Op::And, Some(false), _) | (Op::And, _, Some(false)) => Some(false),
                (Op::Or, Some(true), _) | (Op::Or, _, Some(true)) => Some(true),
                (Op::And, Some(left), Some(right)) => Some(left && right),
                (Op::Or, Some(left), Some(right)) => Some(left || right),
                (Op::Equal, Some(left), Some(right)) => Some(left == right),
                (Op::Different, Some(left), Some(right)) => Some(left != right),
                _ => None,
            },
        }
    }

    pub(crate) fn refine(&self, values: &mut BTreeMap<LocalId, bool>, wanted: bool) -> bool {
        if let Some(value) = self.get(values) {
            return value == wanted;
        }
        match self {
            Self::Local(id) => {
                values.insert(*id, wanted);
                true
            }
            Self::Not(value) => value.refine(values, !wanted),
            Self::Binary(Op::And, left, right) if wanted => {
                left.refine(values, true) && right.refine(values, true)
            }
            Self::Binary(Op::Or, left, right) if !wanted => {
                left.refine(values, false) && right.refine(values, false)
            }
            Self::Binary(op, left, right) => {
                let pair = match (left.get(values), right.get(values)) {
                    (Some(value), None) => Some((value, right)),
                    (None, Some(value)) => Some((value, left)),
                    _ => None,
                };
                match (op, pair) {
                    (Op::And, Some((true, value))) => value.refine(values, wanted),
                    (Op::Or, Some((false, value))) => value.refine(values, wanted),
                    (Op::Equal, Some((known, value))) => value.refine(values, known == wanted),
                    (Op::Different, Some((known, value))) => value.refine(values, known != wanted),
                    _ => true,
                }
            }
            _ => true,
        }
    }
}

impl Graph<'_> {
    pub(crate) fn emission_bool(&mut self, expr: &Expr) -> Result<Option<Value>> {
        if self.proofs.carried.is_empty() || expr.ty != Type::Bool {
            return Ok(None);
        }
        self.emission_value(expr, 0).map(Some)
    }

    pub(crate) fn emission_value(&mut self, expr: &Expr, depth: usize) -> Result<Value> {
        self.charge(1)?;
        if depth >= 256 {
            return Err(Self::budget());
        }
        Ok(match &expr.kind {
            ExprKind::Bool(value) => Value::Known(*value),
            ExprKind::Local(id) if expr.ty == Type::Bool => Value::Local(*id),
            ExprKind::Unary { op, value } if op == "!" => {
                self.emission_value(value, depth + 1)?.negated()
            }
            ExprKind::Coerce { value } if value.ty == Type::Bool => {
                self.emission_value(value, depth + 1)?
            }
            ExprKind::Binary {
                op, left, right, ..
            } if left.ty == Type::Bool
                && right.ty == Type::Bool
                && matches!(op.as_str(), "&&" | "||" | "==" | "!=") =>
            {
                let left = self.emission_value(left, depth + 1)?;
                let right = self.emission_value(right, depth + 1)?;
                if matches!(left, Value::Unknown) || matches!(right, Value::Unknown) {
                    Value::Unknown
                } else {
                    let op = match op.as_str() {
                        "&&" => Op::And,
                        "||" => Op::Or,
                        "==" => Op::Equal,
                        _ => Op::Different,
                    };
                    Value::Binary(op, Box::new(left), Box::new(right))
                }
            }
            _ => Value::Unknown,
        })
    }
}
