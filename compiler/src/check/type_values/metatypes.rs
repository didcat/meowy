use crate::ast::{Expr, TypeExpr, TypeKind};
use crate::check::{Checker, Result, Spec, Value};
use crate::foundation::Module;

impl Checker {
    pub(crate) fn meta_annotation(&mut self, expr: &TypeExpr) -> Result<bool> {
        let TypeKind::Name(name) = &expr.kind else {
            return Ok(false);
        };
        let found = if let Some((module, member)) = name.split_once('.') {
            match self.value(module, expr.span).ok() {
                Some(Value::Module(Module::Core)) => member == "Type",
                Some(Value::FileModule { id, .. }) => self
                    .exports
                    .get(&id)
                    .and_then(|module| module.types.get(member))
                    .is_some_and(|spec| matches!(spec, Spec::Meta)),
                _ => false,
            }
        } else {
            self.scopes
                .iter()
                .rev()
                .find_map(|scope| scope.types.get(name))
                .is_some_and(|spec| matches!(spec, Spec::Meta))
        };
        if found {
            return self.spec(expr).map(|spec| matches!(spec, Spec::Meta));
        }
        Ok(false)
    }

    pub(crate) fn meta_binding(&mut self, expr: &Expr, annotation: &TypeExpr) -> Result<Value> {
        self.type_work.as_mut().unwrap().node(annotation.span)?;
        let value = self.type_binding(expr, None)?;
        if !matches!(value, Value::Type(_)) {
            return Err(Self::error(
                "E207",
                "core.Type binding requires a compile-time type value",
                expr.span,
            ));
        }
        if let Some(model) = &mut self.documentation {
            model.spend("core.Type".len() + 1)?;
            model
                .types
                .insert(annotation.span.start, "core.Type".into());
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub(crate) fn typed_type_bindings_keep_concrete_identity_without_runtime_storage() {
        let source = "c:@\"core\";<Kind>:<c.Type>;<T>:{element<Kind>:<int32>;copy<Type>:element;selected<c.Type>:{|true|->copy;|false|-><string>};-><(selected)[4]>}";
        let program = crate::compile(source).unwrap();
        assert!(program.locals.is_empty());
        assert!(program.body.stmts.is_empty());
        assert!(program.functions.is_empty());
        crate::compile(&format!("{source};v<T>:[1,2,3,4]")).unwrap();
        crate::compile("<Type>:<uint8>;<T>:{n<Type>:4;-><int32[n]>}").unwrap();
    }

    #[test]
    pub(crate) fn typed_type_bindings_check_results_and_leave_other_contexts_gated() {
        for (body, code) in [
            ("kind<Type>:4", "E207"),
            ("kind<Type>:true", "E207"),
            ("kind<Type>:{->4}", "E207"),
            ("kind<Type>:{->field:4}", "E207"),
            ("kind<Type>:missing", "E201"),
            ("kind<Type>:{-><int32>;tail:1/0}", "E107"),
            ("kind<Type>:<int32>;n:kind", ""),
            ("kind<Type>:=<int32>", "B001"),
            ("kind<Type>:<Type>", "B001"),
        ] {
            let source = format!("<T>:{{{body};-><int32>}}");
            let result = crate::compile(&source);
            if code.is_empty() {
                assert!(result.is_ok(), "{result:?}");
            } else {
                assert_eq!(result.unwrap_err()[0].code, code, "{body}");
            }
        }
        assert_eq!(
            crate::compile("kind<Type>:<int32>").unwrap_err()[0].code,
            "B001"
        );
        assert_eq!(
            crate::compile("<T>:{|true|kind<Type>:<int32>;->kind}").unwrap_err()[0].code,
            "E201"
        );
    }

    #[test]
    pub(crate) fn typed_type_bindings_document_the_metatype_annotation() {
        let source = "c:@\"core\";#| Kind. |#<Kind>:<c.Type>;#| Items. |#<T>:{#| Element. |#element<Kind>:<uint8>;-><(element)[4]>}";
        let (_, model) = crate::documentation::checked(source, true).unwrap();
        let model = model.unwrap();
        for (name, signature) in [
            ("Kind", "core.Type"),
            ("element", "core.Type"),
            ("T", "uint8[4]"),
        ] {
            let entry = model
                .entries
                .iter()
                .find(|entry| entry.name == name)
                .unwrap();
            assert!(entry.checked);
            assert_eq!(entry.signature, signature);
        }
    }
}

#[cfg(test)]
mod integration {
    use super::*;
    use crate::ast::StmtKind;
    use crate::check::type_values::{MAX_NODES, MAX_WORK, Work};

    #[test]
    pub(crate) fn metatype_bindings_charge_annotation_and_payload_without_data_locals() {
        let parsed = crate::parser::parse("kind<Type>:<int32>").unwrap();
        let StmtKind::Bind { value, ty, .. } = &parsed.stmts[0].kind else {
            panic!("binding")
        };
        let mut checker = Checker::new();
        checker.type_work = Some(Work::default());
        assert!(matches!(
            checker.type_binding(value, ty.as_ref()).unwrap(),
            Value::Type(_)
        ));
        assert_eq!(checker.type_work.as_ref().unwrap().visits, 1);
        assert_eq!(checker.type_work.as_ref().unwrap().nodes, 2);
        for (visits, nodes, accepted) in [
            (MAX_WORK - 1, MAX_NODES - 2, true),
            (MAX_WORK, 0, false),
            (0, MAX_NODES - 1, false),
        ] {
            checker.type_work = Some(Work {
                visits,
                nodes,
                depth: 0,
            });
            let result = checker.type_binding(value, ty.as_ref());
            if accepted {
                assert!(result.is_ok(), "{:?}", result.err());
            } else {
                assert_eq!(result.err().unwrap().code, "B001");
            }
            assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
            assert!(checker.locals.is_empty());
        }
    }

    #[test]
    pub(crate) fn metatype_bindings_preserve_selected_scope_and_skipped_annotations() {
        crate::compile("<T>:{|false|unused<Missing>:{->unknown()};kind<Type>:{|true|-><int32>;|false|-><Missing>};->kind};v<T>:7").unwrap();
        for (source, code) in [
            (
                "<T>:{kind<Type>:{-><int32>};copy<Type>:kind;copy<Type>:kind;->copy}",
                "E203",
            ),
            ("<T>:{kind<Type>:{-><int32>;-><string>};->kind}", "E205"),
            ("<T>:{kind<Type>:{|false|-><int32>};->kind}", "E211"),
            ("<T>:{kind<Type>:<int32>;n:kind<>;->kind}", "B001"),
            ("<T>:{kind<Type>:<int32>;->kind};outside:kind", "E201"),
        ] {
            assert_eq!(
                crate::compile(source).unwrap_err()[0].code,
                code,
                "{source}"
            );
        }
        let source = "<T>:{|false|unused:{#| Skipped. |#kind<Type>:<int32>;->kind};-><int32>}";
        assert_eq!(
            crate::documentation::checked(source, true).unwrap_err()[0].code,
            "B001"
        );
    }
}
