use crate::ast::{self, ExprKind};
use crate::check::{
    Checker, Constant, Result, Value,
    inputs::{Input, Leaf, Record},
    type_values::Output,
};
use crate::diagnostic::Diagnostic;
use crate::hir::Type;
use std::collections::BTreeMap;

impl Checker {
    pub(crate) fn record_block(&mut self, expr: &ast::Expr, ty: &Type) -> Result<Value> {
        if !self.record_shape(ty) {
            return Err(Diagnostic::unsupported(
                "required record result shape",
                expr.span,
            ));
        }
        self.type_work.as_mut().unwrap().enter(expr.span)?;
        let result = match &expr.kind {
            ExprKind::Group(value) => self.record_block(value, ty),
            ExprKind::Block(block) => self.required_block(block, Some(ty)),
            _ => unreachable!(),
        };
        self.type_work.as_mut().unwrap().depth -= 1;
        result
    }

    pub(crate) fn required_record_field(
        &mut self,
        name: &str,
        annotation: Option<&ast::TypeExpr>,
        expr: &ast::Expr,
        span: ast::Span,
        output: &mut Output,
    ) -> Result<()> {
        let (index, ty) = self.required_record_slot(name, span, output)?;
        if let Some(annotation) = annotation
            && self.ty(annotation)? != ty
        {
            return Err(Self::error(
                "E207",
                "emission annotation differs from the required slot type",
                span,
            ));
        }
        let value = if matches!(ty, Type::Record { .. }) {
            let mut form = expr;
            while let ExprKind::Group(value) = &form.kind {
                form = value;
            }
            let value = if matches!(form.kind, ExprKind::Block(_)) {
                self.record_block(expr, &ty)?
            } else {
                self.type_record(expr, None)?
            };
            if value.data_type() != Some(ty.clone()) {
                return Err(Self::error(
                    "E207",
                    "record field initializer differs from its expected type",
                    expr.span,
                ));
            }
            value
        } else {
            self.scalar_emission(expr, &ty)?
        };
        self.insert_required_field(name, index, value, true, span, output)
    }

    pub(crate) fn required_record_slot(
        &mut self,
        name: &str,
        span: ast::Span,
        output: &Output,
    ) -> Result<(usize, Type)> {
        let Some(Type::Record { fields, .. }) = &output.ty else {
            unreachable!()
        };
        if !self.flow.spend(fields.len() + name.len() + 1) {
            return Err(super::super::Work::budget(span));
        }
        let (index, field) = fields
            .iter()
            .enumerate()
            .find(|(_, field)| field.name == name)
            .ok_or_else(|| {
                Self::error(
                    "E207",
                    format!("field `{name}` is not in the expected record"),
                    span,
                )
            })?;
        Ok((index, field.ty.clone()))
    }

    pub(crate) fn insert_required_field(
        &mut self,
        name: &str,
        index: usize,
        value: Value,
        bind: bool,
        span: ast::Span,
        output: &mut Output,
    ) -> Result<()> {
        if output.fields.contains_key(&index) {
            return Err(Self::error(
                "E205",
                format!("required record field `{name}` may be emitted twice"),
                span,
            ));
        }
        if bind {
            self.declare(name, value.clone(), span)?;
        }
        output.fields.insert(index, value);
        Ok(())
    }

    pub(crate) fn finish_required_record(
        &mut self,
        output: Output,
        span: ast::Span,
    ) -> Result<Value> {
        let ty = output.ty.unwrap();
        let Type::Record { fields, .. } = &ty else {
            unreachable!()
        };
        if let Some((_, field)) = fields
            .iter()
            .enumerate()
            .find(|(index, _)| !output.fields.contains_key(index))
        {
            return Err(Self::error(
                "E204",
                format!("required record field `{}` is not initialized", field.name),
                span,
            ));
        }
        let mut values = BTreeMap::new();
        for (index, value) in output.fields {
            match value {
                Value::Static {
                    value: Constant::Int(value),
                    ..
                } => {
                    values.insert(vec![index], Leaf::Int(Some(value)));
                }
                Value::Static {
                    value: Constant::Bool(value),
                    ..
                } => {
                    values.insert(vec![index], Leaf::Bool(Some(value)));
                }
                Value::Record { input, .. } => {
                    for (path, value) in input.values {
                        let mut key = vec![index];
                        key.extend(path);
                        values.insert(key, value);
                    }
                }
                _ => unreachable!(),
            }
        }
        self.type_work.as_mut().unwrap().materialize(&ty, span)?;
        Ok(Value::Record {
            ty,
            input: Box::new(Record {
                input: Input {
                    work: 0,
                    error: None,
                    value: None,
                },
                values,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(crate) fn required_record_construction_preserves_leaf_kinds_without_runtime_storage() {
        let source = "<R>:<{enabled<boolean>;width<uint8>}>;<T>:{r<R>:{->width:4;->enabled:width>0};-><int32[r.width]>}";
        let program = crate::compile(source).unwrap();
        assert!(program.body.stmts.is_empty());
        assert!(program.locals.is_empty());
        assert!(program.functions.is_empty());
        let parsed =
            crate::parser::parse("r<{enabled<boolean>;width<uint8>}>:{->width:4;->enabled:false}")
                .unwrap();
        let ast::StmtKind::Bind { value, ty, .. } = &parsed.stmts[0].kind else {
            panic!("binding")
        };
        let mut checker = Checker::new();
        checker.type_work = Some(super::super::super::Work::default());
        let Value::Record { input, .. } = checker.type_binding(value, ty.as_ref()).unwrap() else {
            panic!("record")
        };
        assert_eq!(input.boolean(&[0]).unwrap().value, Some(false));
        assert_eq!(input.field(&[1]).unwrap().value, Some(4));
        assert_eq!(input.input.work, 0);
    }

    #[test]
    pub(crate) fn required_record_construction_checks_slots_kinds_scope_and_tails() {
        for (body, code) in [
            ("->width:4", "E204"),
            ("->width:4;->width:5;->enabled:true", "E205"),
            ("->other:4;->enabled:true", "E207"),
            ("->width<uint16>:4;->enabled:true", "E207"),
            ("->width:256;->enabled:true", "E216"),
            ("->width:4;->enabled:1", "E207"),
            ("->width:=4;->enabled:true", "B001"),
            ("->null;->width:4;->enabled:true", "B001"),
            ("|true|->width:4;->enabled:width>0", "E201"),
            ("->width:4;->enabled:true;unused:1/0", "E107"),
        ] {
            let source = format!(
                "<R>:<{{enabled<boolean>;width<uint8>}}>;<T>:{{r<R>:{{{body}}};-><int32>}}"
            );
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{body}: {error:?}");
        }
    }
    #[test]
    pub(crate) fn required_record_construction_preserves_shape_limits_and_failed_scope() {
        use crate::ast::{Block, Expr, Span, Stmt, StmtKind};
        use crate::hir::Field;
        for depth in [32, 33] {
            let span = Span::new(0, 1);
            let mut ty = Type::Int {
                bits: 32,
                signed: true,
            };
            let mut expr = Expr {
                span,
                kind: ExprKind::Int("1".into()),
            };
            for _ in 0..depth {
                ty = Type::Record {
                    primary: Box::new(Type::Null),
                    fields: vec![Field {
                        name: "n".into(),
                        ty,
                        mutable: false,
                    }],
                };
                expr = Expr {
                    span,
                    kind: ExprKind::Block(Block {
                        span,
                        label: None,
                        stmts: vec![Stmt {
                            span,
                            kind: StmtKind::Emit {
                                label: None,
                                name: Some("n".into()),
                                ty: None,
                                mutable: false,
                                value: expr,
                            },
                        }],
                    }),
                };
            }
            let mut checker = Checker::new();
            checker.type_work = Some(super::super::super::Work::default());
            let scopes = checker.scopes.len();
            let result = checker.record_block(&expr, &ty);
            if depth == 32 {
                assert!(matches!(result.unwrap(), Value::Record { .. }));
            } else {
                assert_eq!(result.err().unwrap().code, "B001");
            }
            assert_eq!(checker.scopes.len(), scopes);
            assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
            assert!(checker.locals.is_empty());
        }
        for count in [256, 257] {
            let fields = (0..count)
                .map(|id| format!("n{id}<int32>;"))
                .collect::<String>();
            let values = (0..count)
                .map(|id| format!("->n{id}:1;"))
                .collect::<String>();
            let source = format!("<R>:<{{{fields}}}>;<T>:{{r<R>:{{{values}}};-><int32>}}");
            let result = crate::compile(&source);
            if count == 256 {
                assert!(result.is_ok(), "{result:?}");
            } else {
                assert_eq!(result.unwrap_err()[0].code, "B001");
            }
        }
    }

    #[test]
    pub(crate) fn required_record_construction_documents_completed_fields() {
        let source = "<R>:<{width<uint8>}>;#| Items. |#<T>:{#| Record. |#r<R>:{#| Width. |#->width:4};-><int32[r.width]>}";
        let (_, model) = crate::documentation::checked(source, true).unwrap();
        let model = model.unwrap();
        let field = model
            .entries
            .iter()
            .find(|entry| {
                entry.name == "width" && entry.kind == crate::documentation::Kind::Emission
            })
            .unwrap();
        assert!(field.checked);
        assert_eq!(field.signature, "uint8");
        assert_eq!(
            model
                .entries
                .iter()
                .find(|entry| entry.name == "T")
                .unwrap()
                .signature,
            "int32[4]"
        );
    }
}
