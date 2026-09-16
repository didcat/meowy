#[cfg(test)]
mod blocks;
mod forms;

use crate::ast::{self, ExprKind};
use crate::check::{Checker, Constant, Result, Value, inputs::Input};
use crate::diagnostic::Diagnostic;
use crate::hir::Type;

impl Checker {
    pub(crate) fn type_boolean(
        &mut self,
        expr: &ast::Expr,
        annotation: Option<&ast::TypeExpr>,
    ) -> Result<Value> {
        self.boolean_form(expr, self.type_work.as_ref().unwrap().depth, &mut 0)?;
        let value = self.required_boolean(expr)?;
        if let Some(annotation) = annotation {
            let ty = self.source_type(annotation, true)?;
            if !matches!(ty, Type::Bool | Type::Int { .. }) {
                return Err(Diagnostic::unsupported(
                    "computed boolean binding annotation",
                    expr.span,
                ));
            }
            Self::expected_value(
                Self::constant_expr(Constant::Bool(value), expr.span),
                &ty,
                expr.span,
            )?;
        }
        Ok(Value::Static {
            value: Constant::Bool(value),
            ty: Type::Bool,
        })
    }

    pub(crate) fn required_boolean(&mut self, expr: &ast::Expr) -> Result<bool> {
        self.type_work.as_mut().unwrap().enter(expr.span)?;
        let result = (|| {
            if !matches!(expr.kind, ExprKind::Group(_) | ExprKind::Block(_)) {
                self.type_work.as_mut().unwrap().logical.charge(1, 0)?;
            }
            let input = match &expr.kind {
                ExprKind::Name(name) => match self.required_value(name, expr.span)? {
                    Value::Static {
                        value: Constant::Bool(value),
                        ..
                    }
                    | Value::Constant(Constant::Bool(value)) => Some(Input {
                        value: Some(value),
                        error: None,
                        work: 0,
                    }),
                    Value::Local {
                        id,
                        ty: Type::Bool,
                        mutable: false,
                        ..
                    } => self.bool_inputs.get(&id).cloned(),
                    Value::FileModule { id, .. } => self.module_boolean(id).cloned(),
                    _ => None,
                },
                ExprKind::Field { .. } => {
                    self.charge_ancestors(expr)?;
                    let (id, ty, path) = self.required_path(expr)?;
                    if ty != Type::Bool {
                        return Err(Diagnostic::unsupported(
                            "computed field without a boolean leaf",
                            expr.span,
                        ));
                    }
                    id.boolean(self, &path)
                }
                ExprKind::Block(_) => {
                    let Value::Static {
                        value: Constant::Bool(value),
                        ..
                    } = self.scalar_block(expr, &Type::Bool)?
                    else {
                        unreachable!()
                    };
                    return Ok(value);
                }
                ExprKind::Group(value) => return self.required_boolean(value),
                ExprKind::Unary { op, value } if op == "!" => {
                    return self.required_boolean(value).map(|value| !value);
                }
                ExprKind::Binary { op, left, right } if matches!(op.as_str(), "&&" | "||") => {
                    let left = self.required_boolean(left)?;
                    if left == (op == "||") {
                        return Ok(left);
                    }
                    return self.required_boolean(right);
                }
                ExprKind::Binary { op, left, right }
                    if matches!(op.as_str(), "==" | "!=" | "<" | ">" | "<=" | ">=") =>
                {
                    if let Some(value) = self.required_comparison(op, left, right)? {
                        return Ok(value);
                    }
                    let left = self.required_boolean(left)?;
                    let right = self.required_boolean(right)?;
                    return Ok(if op == "==" {
                        left == right
                    } else {
                        left != right
                    });
                }
                _ => return Err(self.type_unavailable(expr)?),
            }
            .ok_or_else(|| {
                Self::error(
                    "E211",
                    "boolean initializer is unavailable during required type evaluation",
                    expr.span,
                )
            })?;
            self.type_work.as_mut().unwrap().input(&input, expr.span)?;
            input.value.ok_or_else(|| {
                Self::error(
                    "E211",
                    "required input has no checked boolean value",
                    expr.span,
                )
            })
        })();
        self.type_work.as_mut().unwrap().depth -= 1;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, Span};
    use crate::check::type_values::{MAX_WORK, Work};

    pub(crate) fn checker() -> Checker {
        let mut checker = crate::check::inputs::tests::check("flag:{->false;unused:7}");
        let id = *checker.bool_inputs.keys().last().unwrap();
        checker
            .declare(
                "flag",
                Value::Local {
                    id,
                    ty: Type::Bool,
                    mutable: false,
                    owner: 0,
                    constant: None,
                },
                Span::new(0, 4),
            )
            .unwrap();
        checker
    }

    #[test]
    pub(crate) fn required_booleans_keep_false_values_lexical_names_and_no_storage() {
        let mut checker = checker();
        checker.type_work = Some(Work::default());
        let expr = Expr {
            kind: ExprKind::Name("flag".into()),
            span: Span::new(0, 4),
        };
        assert!(matches!(
            checker.type_scalar(&expr, None).unwrap(),
            Value::Static {
                value: Constant::Bool(false),
                ty: Type::Bool
            }
        ));
        let program = crate::compile(
            "<T>:{flag:true;copy<boolean>:(flag);inner:{flag:false;->flag<>};->copy<>}",
        )
        .unwrap();
        assert!(program.body.stmts.is_empty());
        assert!(program.locals.is_empty());
        crate::compile("true:3;<T>:{copy:true;->copy<>};v<T>:7").unwrap();
        crate::compile("flag:false;f<boolean>:(){<T>:{copy:flag;->copy<>};v<T>:true;->v}").unwrap();
    }

    #[test]
    pub(crate) fn required_booleans_keep_kind_scope_purity_and_operator_gates() {
        for (source, code) in [
            ("<T>:{flag<int32>:true;-><int32>}", "E207"),
            ("<T>:{flag<boolean>:1;-><int32>}", "E207"),
            ("<T>:{flag:true;-><int32[flag]>}", "B001"),
            ("<T>:{flag:true;->flag}", "E211"),
            ("<T>:{flag:true;->flag<>};v:flag", "E201"),
            ("<T>:{flag:true;flag:false;-><int32>}", "E203"),
            ("flag:=true;<T>:{copy:flag;-><int32>}", "E211"),
            (
                "d:@\"debug\";flag:{->true;d.print(9)};<T>:{copy:flag;-><int32>}",
                "E211",
            ),
            (
                "f<boolean>:(flag<boolean>){<T>:{copy:flag;-><int32>};->flag}",
                "E211",
            ),
            (
                "flag:true;f<boolean>:(){<T>:{copy:flag;->copy<>};->flag}",
                "B001",
            ),
            ("<T>:{flag:-true;-><int32>}", "B001"),
            ("<T>:{flag:true<false;-><int32>}", "B001"),
            ("<T>:{flag:true;|flag|{-><int32>}}", "B001"),
        ] {
            let error = crate::compile(source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{source}: {error:?}");
        }
    }

    #[test]
    pub(crate) fn required_booleans_charge_cached_work_and_retain_first_errors() {
        let mut checker = checker();
        let cost = checker.bool_inputs.values().last().unwrap().work;
        checker.type_work = Some(Work {
            visits: MAX_WORK - cost - 1,
            ..Work::default()
        });
        let expr = Expr {
            kind: ExprKind::Name("flag".into()),
            span: Span::new(0, 4),
        };
        assert!(!checker.required_boolean(&expr).unwrap());
        assert_eq!(checker.type_work.as_ref().unwrap().visits, MAX_WORK);
        let error = checker.required_boolean(&expr).unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("computed type bootstrap budget"));
        let source = "row:{->n<uint8>:255};flag:row.n+1==0;<T>:{copy:flag;-><int32>}";
        let error = crate::compile(source).unwrap_err().remove(0);
        assert_eq!(error.code, "E107");
        assert_eq!(error.span.start, source.find("row.n+1").unwrap());
    }
    #[test]
    pub(crate) fn required_boolean_groups_restore_depth_after_failure() {
        use crate::check::type_values::MAX_DEPTH;
        for groups in [MAX_DEPTH - 1, MAX_DEPTH] {
            let mut checker = Checker::new();
            checker.type_work = Some(Work::default());
            let literal = Expr {
                kind: ExprKind::Name("false".into()),
                span: Span::new(0, 5),
            };
            let mut expr = literal.clone();
            for _ in 0..groups {
                expr = Expr {
                    span: expr.span,
                    kind: ExprKind::Group(Box::new(expr)),
                };
            }
            let result = checker.type_scalar(&expr, None);
            if groups == MAX_DEPTH - 1 {
                assert!(matches!(
                    result.unwrap(),
                    Value::Static {
                        value: Constant::Bool(false),
                        ..
                    }
                ));
            } else {
                assert_eq!(result.err().unwrap().code, "B001");
            }
            assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
            assert!(!checker.required);
            assert!(!checker.required_boolean(&literal).unwrap());
        }
    }
    pub(crate) fn expression(source: &str) -> Expr {
        let block = crate::parser::parse(&format!("value:{source}")).unwrap();
        let crate::ast::StmtKind::Bind { value, .. } = &block.stmts[0].kind else {
            panic!("binding")
        };
        value.clone()
    }

    #[test]
    pub(crate) fn required_boolean_logic_keeps_truth_values_and_no_runtime_storage() {
        for a in [false, true] {
            for b in [false, true] {
                for (source, value) in [
                    (format!("!{a}"), !a),
                    (format!("{a}&&{b}"), a && b),
                    (format!("{a}||{b}"), a || b),
                    (format!("!({a}&&(!{b}||{a}))"), !(a && (!b || a))),
                ] {
                    let mut checker = Checker::new();
                    checker.type_work = Some(Work::default());
                    assert!(
                        matches!(checker.type_scalar(&expression(&source), None).unwrap(),
                            Value::Static { value: Constant::Bool(found), ty: Type::Bool } if found == value
                        ),
                        "{source}"
                    );
                }
            }
        }
        let program =
            crate::compile("<T>:{true:false;flag:!true;copy:flag&&true;->copy<>}").unwrap();
        assert!(program.body.stmts.is_empty());
        assert!(program.locals.is_empty());
    }

    #[test]
    pub(crate) fn required_boolean_logic_skips_retained_errors_and_source_work() {
        for (source, value) in [("false&&flag", false), ("true||flag", true)] {
            let mut checker = checker();
            let input = checker.bool_inputs.values_mut().last().unwrap();
            input.error = Some(Checker::error(
                "E107",
                "retained error",
                Span::new(100, 104),
            ));
            input.value = None;
            input.work = MAX_WORK;
            checker.type_work = Some(Work::default());
            assert!(
                matches!(checker.type_scalar(&expression(source), None).unwrap(),
                    Value::Static { value: Constant::Bool(found), .. } if found == value
                )
            );
            assert_eq!(checker.type_work.as_ref().unwrap().visits, 2);
        }
        let source = "row:{->n<uint8>:255};flag:row.n+1==0;<T>:{copy:true&&flag;->copy<>}";
        let error = crate::compile(source).unwrap_err().remove(0);
        assert_eq!(error.code, "E107");
        assert_eq!(error.span.start, source.find("row.n+1").unwrap());
    }
    #[test]
    pub(crate) fn required_boolean_equality_keeps_values_and_charges_both_reads() {
        for a in [false, true] {
            for b in [false, true] {
                for (op, value) in [("==", a == b), ("!=", a != b)] {
                    let mut checker = Checker::new();
                    checker.type_work = Some(Work::default());
                    assert!(
                        matches!(checker.type_scalar(&expression(&format!("{a}{op}{b}")), None).unwrap(),
                            Value::Static { value: Constant::Bool(found), .. } if found == value
                        )
                    );
                }
            }
        }
        let mut checker = checker();
        let cost = checker.bool_inputs.values().last().unwrap().work;
        checker.type_work = Some(Work::default());
        assert!(matches!(
            checker
                .type_scalar(&expression("flag==flag"), None)
                .unwrap(),
            Value::Static {
                value: Constant::Bool(true),
                ..
            }
        ));
        assert_eq!(checker.type_work.as_ref().unwrap().visits, 3 + 2 * cost);
    }

    #[test]
    pub(crate) fn required_boolean_equality_retains_the_first_evaluated_error() {
        for (value, failed) in [
            ("a==b", "row.a+1"),
            ("b!=a", "row.b+2"),
            ("false==a", "row.a+1"),
            ("true!=b", "row.b+2"),
        ] {
            let source = format!(
                "row:{{->a<uint8>:255;->b<uint8>:254}};a:row.a+1==0;b:row.b+2==0;<T>:{{flag:{value};->flag<>}}"
            );
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, "E107");
            assert_eq!(error.span.start, source.find(failed).unwrap(), "{value}");
        }
    }
}
