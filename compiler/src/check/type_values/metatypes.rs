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
