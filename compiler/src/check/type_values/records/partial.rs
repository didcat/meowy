use crate::ast::{Expr, ExprKind};
use crate::check::{Checker, Result, Value};
use crate::diagnostic::Diagnostic;
use crate::hir::Type;

impl Checker {
    pub(crate) fn partial_record(&mut self, expr: &Expr, ty: &Type) -> Result<Value> {
        self.type_work.as_mut().unwrap().enter(expr.span)?;
        let result = (|| {
            let block = match &expr.kind {
                ExprKind::Group(value) => return self.partial_record(value, ty),
                ExprKind::Block(block) => block,
                _ => unreachable!(),
            };
            let mut output = self.required_output(block, Some(ty))?;
            if output.fields.is_empty() {
                return Err(Diagnostic::unsupported(
                    "empty required composition sources",
                    expr.span,
                ));
            }
            let Some(Type::Record { fields, primary }) = output.ty.take() else {
                unreachable!()
            };
            let fields = fields
                .into_iter()
                .enumerate()
                .filter_map(|(index, field)| output.fields.contains_key(&index).then_some(field))
                .collect();
            output.ty = Some(Type::Record { primary, fields });
            output.fields = output.fields.into_values().enumerate().collect();
            self.finish_required_record(output, block.span)
        })();
        self.type_work.as_mut().unwrap().depth -= 1;
        result
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub(crate) fn inline_composition_keeps_expected_types_and_complete_nested_fields() {
        let source = "<R>:<{enabled<boolean>;part<{width<uint8>}>;size<uint8>}>;<T>:{r<R>:{->({local<uint8>:4;->size:local;->part:{->width:size}});->enabled:true};|r.enabled|-><int32[r.part.width+r.size]>;|!r.enabled|-><string>}";
        let program = crate::compile(source).unwrap();
        assert!(program.body.stmts.is_empty());
        assert!(program.locals.is_empty());
        assert!(program.functions.is_empty());
        crate::compile(&format!("{source};v<T>:[1,2,3,4,5,6,7,8]")).unwrap();
    }

    #[test]
    pub(crate) fn inline_composition_preserves_slot_scope_and_source_failures() {
        for (body, code) in [
            ("->{->width:4}", "E204"),
            ("->{->width:4};->width:5;->enabled:true", "E205"),
            ("->width:4;->{->width:5};->enabled:true", "E205"),
            ("->{->width:4};->{->enabled:true}", "E205"),
            ("->{->width:4;->width:5};->enabled:true", "E205"),
            ("->{->absent:4};->enabled:true", "E207"),
            ("->{->width<uint16>:4};->enabled:true", "E207"),
            ("->{->width:256};->enabled:true", "E216"),
            ("->{->width:true};->enabled:true", "E207"),
            ("->{->width:4};->enabled:width>0", "E201"),
            ("->{local:4;->width:4};->enabled:local>0", "E201"),
            ("->{|true|->width:4;->enabled:width>0}", "E201"),
            ("->{->width:4;unused:1/0};->enabled:true", "E107"),
            ("->{};->width:4;->enabled:true", "B001"),
            ("->{|false|->width:4};->width:4;->enabled:true", "B001"),
            ("->{->null;->width:4};->enabled:true", "B001"),
            ("->{->width:=4};->enabled:true", "B001"),
        ] {
            let source = format!(
                "<R>:<{{enabled<boolean>;width<uint8>}}>;<T>:{{r<R>:{{{body}}};-><int32>}}"
            );
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{body}: {error:?}");
        }
        let source = "<R>:<{part<{a<uint8>;b<uint8>}>}>;<T>:{r<R>:{->{->part:{->a:4}}};-><int32>}";
        assert_eq!(crate::compile(source).unwrap_err()[0].code, "E204");
    }
}
