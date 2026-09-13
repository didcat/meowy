use crate::ast::{Expr, ExprKind, Span};
use crate::check::{Checker, Constant, Result, Value, type_values::Output};
use crate::diagnostic::Diagnostic;
use crate::hir::Type;

impl Checker {
    pub(crate) fn compose_required_record(
        &mut self,
        expr: &Expr,
        span: Span,
        output: &mut Output,
    ) -> Result<()> {
        let mut form = expr;
        while let ExprKind::Group(value) = &form.kind {
            form = value;
        }
        let ty = match &form.kind {
            ExprKind::Name(name) => self.required_value(name, form.span)?.data_type(),
            ExprKind::Field { .. } => Some(self.required_path(form)?.1),
            _ => None,
        };
        if !matches!(ty, Some(Type::Record { .. })) {
            return Err(Diagnostic::unsupported(
                "required primary composition outside existing records",
                span,
            ));
        }
        let Value::Record { ty, input } = self.type_record(expr, None)? else {
            unreachable!()
        };
        if output
            .value
            .replace(Value::Constant(Constant::Null))
            .is_some()
        {
            return Err(Self::error(
                "E205",
                "required record primary may be emitted twice",
                span,
            ));
        }
        let Type::Record { fields, .. } = ty else {
            unreachable!()
        };
        for (index, field) in fields.into_iter().enumerate() {
            let (slot, expected) = self.required_record_slot(&field.name, span, output)?;
            if expected != field.ty {
                return Err(Self::error(
                    "E207",
                    format!(
                        "composed field `{}` has type {:?}, expected {expected:?}",
                        field.name, field.ty
                    ),
                    span,
                ));
            }
            self.type_work.as_mut().unwrap().spend(span)?;
            let missing = || Self::error("E211", "composed field has no checked value", span);
            let value = match &field.ty {
                Type::Int { .. } => Value::Static {
                    value: Constant::Int(
                        input
                            .field(&[index])
                            .and_then(|input| input.value)
                            .ok_or_else(missing)?,
                    ),
                    ty: field.ty.clone(),
                },
                Type::Bool => Value::Static {
                    value: Constant::Bool(
                        input
                            .boolean(&[index])
                            .and_then(|input| input.value)
                            .ok_or_else(missing)?,
                    ),
                    ty: Type::Bool,
                },
                Type::Record { .. } => {
                    let mut part = input.project(&[index]).ok_or_else(missing)?;
                    part.input.work = 0;
                    self.type_work
                        .as_mut()
                        .unwrap()
                        .materialize(&field.ty, span)?;
                    Value::Record {
                        ty: field.ty.clone(),
                        input: Box::new(part),
                    }
                }
                _ => unreachable!(),
            };
            self.insert_required_field(&field.name, slot, value, false, span, output)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub(crate) fn required_composition_maps_partial_nested_fields_without_runtime_storage() {
        let source = "<R>:<{enabled<boolean>;part<{width<uint8>}>}>;<T>:{base<{part<{width<uint8>}>}>:{->part:{->width:4}};r<R>:{->base;->enabled:false};|r.enabled|-><string>;|!r.enabled|-><int32[r.part.width]>}";
        let program = crate::compile(source).unwrap();
        assert!(program.body.stmts.is_empty());
        assert!(program.locals.is_empty());
        assert!(program.functions.is_empty());
        crate::compile(&format!("{source};v<T>:[1,2,3,4]")).unwrap();
    }

    #[test]
    pub(crate) fn required_composition_preserves_primary_collisions_and_lexical_bindings() {
        for (body, code) in [
            ("->part", "E204"),
            ("->missing;->enabled:true", "E201"),
            ("->part;->enabled:width>0", "E201"),
            ("->part;->width:4;->enabled:true", "E205"),
            ("->width:4;->part;->enabled:true", "E205"),
            ("->part;->other", "E205"),
            ("->extra;->enabled:true", "E207"),
            ("->wide;->enabled:true", "E207"),
            ("->{->width:4};->enabled:true", "B001"),
        ] {
            let source = format!(
                "part:{{->width<uint8>:4}};other:{{->enabled:true}};extra:{{->absent:4}};wide:{{->width<uint16>:4}};<R>:<{{enabled<boolean>;width<uint8>}}>;<T>:{{r<R>:{{{body}}};-><int32>}}"
            );
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{body}: {error:?}");
        }
        let ordinary = "a:{->x:1};b:{->y:2};r<{x<int32>;y<int32>}>:{->a;->b}";
        assert_eq!(crate::compile(ordinary).unwrap_err()[0].code, "E205");
    }
    #[test]
    pub(crate) fn required_composition_charges_one_source_and_each_forwarded_field() {
        use super::*;
        use crate::check::type_values::Work;
        let mut checker = crate::check::inputs::tests::check("->source:{->a<uint8>:4;->b:false}");
        let id = checker.module.inputs["source"].id;
        let ty = checker.locals[id].clone();
        let cost = checker.record_inputs[&id].input.work;
        let locals = checker.locals.len();
        let span = Span::new(0, 6);
        checker
            .declare(
                "source",
                Value::Local {
                    id,
                    ty: ty.clone(),
                    mutable: false,
                    owner: 0,
                    constant: None,
                },
                span,
            )
            .unwrap();
        checker.type_work = Some(Work::default());
        let mut output = Output {
            ty: Some(ty),
            ..Output::default()
        };
        checker
            .compose_required_record(
                &Expr {
                    span,
                    kind: ExprKind::Name("source".into()),
                },
                span,
                &mut output,
            )
            .unwrap();
        assert_eq!(checker.type_work.as_ref().unwrap().visits, cost + 3);
        assert_eq!(checker.record_inputs[&id].input.work, cost);
        assert_eq!(checker.locals.len(), locals);
        assert!(matches!(
            output.value,
            Some(Value::Constant(Constant::Null))
        ));
        assert!(matches!(
            output.fields.get(&1),
            Some(Value::Static {
                value: Constant::Bool(false),
                ..
            })
        ));
        assert_eq!(
            checker.required_value("a", span).err().unwrap().code,
            "E201"
        );
    }

    #[test]
    pub(crate) fn required_composition_documents_result_shapes_without_inventing_bindings() {
        let source = "base:{->width<uint8>:4};<R>:<{width<uint8>;enabled<boolean>}>;#| Items. |#<T>:{#| Settings. |#r<R>:{->base;->enabled:true};-><int32[r.width]>}";
        let (_, model) = crate::documentation::checked(source, true).unwrap();
        let model = model.unwrap();
        assert_eq!(
            model
                .entries
                .iter()
                .find(|entry| entry.name == "T")
                .unwrap()
                .signature,
            "int32[4]"
        );
        let record = model
            .entries
            .iter()
            .find(|entry| entry.name == "r")
            .unwrap();
        assert!(record.checked);
        assert!(record.signature.contains("width<uint8>"));
    }
}
