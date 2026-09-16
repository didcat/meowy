use super::{Checker, Constant, Result, Spec, Value};
use crate::ast::{self, ExprKind, Span, TypeKind};
use crate::diagnostic::Diagnostic;
use crate::foundation::{Descriptor, Item, Module};
use crate::hir::{self, Type};
use std::collections::BTreeMap;

impl Checker {
    pub(crate) fn primitive(name: &str) -> Option<Type> {
        Some(match name {
            "null" => Type::Null,
            "never" => Type::Never,
            "boolean" => Type::Bool,
            "string" => Type::String,
            "isize" => Type::Int {
                bits: 64,
                signed: true,
            },
            "usize" => Type::Int {
                bits: 64,
                signed: false,
            },
            "float32" => Type::Float { bits: 32 },
            "float64" => Type::Float { bits: 64 },
            value => {
                let (digits, signed) = if let Some(digits) = value.strip_prefix("uint") {
                    (digits, false)
                } else {
                    let digits = value.strip_prefix("int")?;
                    (digits, true)
                };
                let bits = digits.parse().ok()?;
                if ![8, 16, 32, 64].contains(&bits) {
                    return None;
                }
                Type::Int { bits, signed }
            }
        })
    }

    pub(crate) fn value(&self, name: &str, span: Span) -> Result<Value> {
        let value = self
            .scopes
            .iter()
            .rev()
            .find_map(|scope| scope.values.get(name))
            .cloned()
            .ok_or_else(|| Self::error("E201", format!("unknown value `{name}`"), span))?;
        match &value {
            Value::Pending(id) if self.queries[*id].owner != self.owner => Err(Self::error(
                "E223",
                "proof descriptors cannot be captured by another function",
                span,
            )),
            Value::Local { .. } if self.required => Ok(value),
            Value::Local { owner, .. } | Value::Control { owner, .. } if *owner != self.owner => {
                Err(Diagnostic::unsupported(
                    "capturing a value from an enclosing function or module",
                    span,
                ))
            }
            _ => Ok(value),
        }
    }

    pub(crate) fn declare(&mut self, name: &str, value: Value, span: Span) -> Result<()> {
        let scope = self.scopes.last_mut().expect("scope");
        if scope.values.contains_key(name) {
            return Err(Self::error(
                "E203",
                format!("value `{name}` is already declared in this scope"),
                span,
            ));
        }
        scope.values.insert(name.into(), value);
        if self.documentation.is_some() {
            scope.doc_values.insert(name.into(), span.start);
        }
        Ok(())
    }

    pub(crate) fn local(&mut self, ty: Type) -> usize {
        let id = self.locals.len();
        if ty.has_mutable_fields() {
            self.proofs.fields.insert(id);
        }
        self.locals.push(ty);
        self.proofs.bindings.insert(id, self.reach);
        id
    }

    pub(crate) fn spec(&mut self, expr: &ast::TypeExpr) -> Result<Spec> {
        self.source_spec(expr, false)
    }

    pub(crate) fn source_spec(&mut self, expr: &ast::TypeExpr, charge: bool) -> Result<Spec> {
        if charge
            && matches!(
                expr.kind,
                TypeKind::Function { .. }
                    | TypeKind::Record { .. }
                    | TypeKind::Union(_)
                    | TypeKind::List { .. }
                    | TypeKind::Reference { .. }
            )
        {
            self.type_work.as_mut().unwrap().logical.charge(1, 1)?;
        }
        let spec = self.source_spec_inner(expr, charge)?;
        if charge && matches!(expr.kind, TypeKind::Name(_) | TypeKind::Computed(_)) {
            self.type_work.as_mut().unwrap().logical_spec(&spec)?;
        }
        Ok(spec)
    }

    pub(crate) fn source_type(&mut self, expr: &ast::TypeExpr, charge: bool) -> Result<Type> {
        let spec = self.source_spec(expr, charge)?;
        self.spec_type(spec, expr.span)
    }

    pub(crate) fn source_spec_inner(&mut self, expr: &ast::TypeExpr, charge: bool) -> Result<Spec> {
        match &expr.kind {
            TypeKind::Name(name) => {
                if let Some((module, member)) = name.split_once('.') {
                    if let Value::FileModule { id, .. } = self.value(module, expr.span)? {
                        return self.module_type(id, member, expr.span);
                    }
                    if let Value::Module(module) = self.value(module, expr.span)?
                        && module == Module::Core
                    {
                        if member == "Type" {
                            return Ok(Spec::Meta);
                        }
                        return Self::primitive(member).map(Spec::Data).ok_or_else(|| {
                            if ["int128", "uint128", "Type", "error", "any"].contains(&member) {
                                Diagnostic::unsupported(format!("type `core.{member}`"), expr.span)
                            } else {
                                Self::error(
                                    "E202",
                                    format!("unknown core type `{member}`"),
                                    expr.span,
                                )
                            }
                        });
                    }
                    if let Value::Module(module) = self.value(module, expr.span)? {
                        if let Some(ty) = Descriptor::resolve(module, member) {
                            return Ok(Spec::Descriptor(ty));
                        }
                        if let Some(Item::Type(ty)) = module.item(member) {
                            return Ok(Spec::Data(Type::Foundation(ty)));
                        }
                        if module.partial() {
                            return Err(Diagnostic::unsupported(
                                format!("type member `{}.{member}`", module.name()),
                                expr.span,
                            ));
                        }
                    }
                    return Err(Self::error(
                        "E202",
                        format!("unknown type `{name}`"),
                        expr.span,
                    ));
                }
                self.scopes
                    .iter()
                    .rev()
                    .find_map(|scope| scope.types.get(name))
                    .cloned()
                    .ok_or_else(|| {
                        if ["int128", "uint128", "Type", "error", "any"].contains(&name.as_str()) {
                            Diagnostic::unsupported(format!("type `{name}`"), expr.span)
                        } else {
                            Self::error("E202", format!("unknown type `{name}`"), expr.span)
                        }
                    })
            }
            TypeKind::Function { params, result } => {
                let params = params
                    .iter()
                    .map(|ty| self.source_function(ty, charge))
                    .collect::<Result<Vec<_>>>()?;
                let result = self.source_function(result, charge)?;
                Ok(Spec::Function { params, result })
            }
            TypeKind::Record { primary, fields } => {
                if charge && primary.is_none() {
                    self.type_work.as_mut().unwrap().logical.charge(1, 1)?;
                }
                let primary = primary
                    .as_ref()
                    .map(|ty| self.source_type(ty, charge))
                    .transpose()?
                    .unwrap_or(Type::Null);
                let mut result = BTreeMap::new();
                for (name, ty, mutable) in fields {
                    let ty = self.source_type(ty, charge)?;
                    if *mutable && ty.has_reference() && !ty.fixed_borrowed_value() {
                        return Err(Diagnostic::unsupported(
                            "mutable reference-bearing record fields",
                            expr.span,
                        ));
                    }
                    if result
                        .insert(
                            name.clone(),
                            hir::Field {
                                name: name.clone(),
                                ty,
                                mutable: *mutable,
                            },
                        )
                        .is_some()
                    {
                        return Err(Self::error(
                            "E206",
                            format!("duplicate record field `{name}`"),
                            expr.span,
                        ));
                    }
                }
                if matches!(primary, Type::Record { .. }) {
                    return Err(Diagnostic::unsupported(
                        "composed primary record types",
                        expr.span,
                    ));
                }
                let ty = Type::Record {
                    primary: Box::new(primary),
                    fields: result.into_values().collect(),
                };
                Ok(Spec::Data(ty))
            }
            TypeKind::Computed(value) => {
                if self.pending_type(value)? {
                    return Ok(Spec::Descriptor(Descriptor::Result));
                }
                Ok(Spec::Data(self.type_value(value)?))
            }
            TypeKind::Union(types) => {
                let types = types
                    .iter()
                    .map(|ty| {
                        let spec = self.source_spec(ty, charge)?;
                        if matches!(spec, Spec::Descriptor(_)) {
                            return Err(Diagnostic::unsupported(
                                "proof descriptor union construction",
                                ty.span,
                            ));
                        }
                        self.spec_type(spec, ty.span)
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(Spec::Data(Type::union(types)))
            }
            TypeKind::List { element, size } => {
                let Some(size) = size else {
                    return Err(Diagnostic::unsupported("borrowed slice types", expr.span));
                };
                let element = self.source_type(element, charge)?;
                let capacity = self.list_extent(size)?;
                Ok(Spec::Data(self.list_type(element, capacity, expr.span)?))
            }
            TypeKind::Reference { value, mutable } => {
                if *mutable {
                    let ty = self.source_type(value, charge)?;
                    return Ok(Spec::Data(self.exclusive_type(ty, expr.span)?));
                }
                let ty = self.source_type(value, charge)?;
                Ok(Spec::Data(self.reference_type(ty, expr.span)?))
            }
            TypeKind::Unsupported(feature) => Err(Diagnostic::unsupported(feature, expr.span)),
        }
    }

    pub(crate) fn construct_type(&mut self, expr: &ast::TypeExpr) -> Result<Type> {
        self.construction_root(expr.span, |checker| checker.source_type(expr, true))
    }

    #[cfg(test)]
    pub(crate) fn ty(&mut self, expr: &ast::TypeExpr) -> Result<Type> {
        let spec = self.spec(expr)?;
        self.spec_type(spec, expr.span)
    }

    pub(crate) fn source_function(&mut self, expr: &ast::TypeExpr, charge: bool) -> Result<Type> {
        let spec = self.source_spec(expr, charge)?;
        if matches!(spec, Spec::Descriptor(_)) {
            return Err(Diagnostic::unsupported(
                "proof descriptor function signatures",
                expr.span,
            ));
        }
        if matches!(spec, Spec::Meta) {
            return Err(Diagnostic::unsupported(
                "type-producing function signatures",
                expr.span,
            ));
        }
        self.spec_type(spec, expr.span)
    }

    pub(crate) fn type_literal(&mut self, expr: &ast::TypeExpr) -> Result<Type> {
        let spec = self.spec(expr)?;
        self.literal_type(spec, expr.span)
    }

    pub(crate) fn literal_type(&mut self, spec: Spec, span: Span) -> Result<Type> {
        if let Spec::Descriptor(ty) = spec {
            return Err(Diagnostic::unsupported(
                format!("first-class {} type values", ty.name()),
                span,
            ));
        }
        if matches!(spec, Spec::Meta) {
            return Err(Diagnostic::unsupported(
                "first-class core.Type values",
                span,
            ));
        }
        self.spec_type(spec, span)
    }

    pub(crate) fn spec_type(&mut self, spec: Spec, span: Span) -> Result<Type> {
        match spec {
            Spec::Descriptor(ty) => Err(Self::error(
                "E223",
                format!(
                    "{} is a compile-time-only descriptor with no runtime representation",
                    ty.name()
                ),
                span,
            )),
            Spec::Meta => Err(Self::error(
                "E211",
                "core.Type has no runtime representation",
                span,
            )),
            Spec::Data(ty) if ty.has_drop() => Err(Diagnostic::unsupported(
                "storage requiring owning cleanup schedules",
                span,
            )),
            Spec::Data(ty) => {
                if let Some(model) = &mut self.documentation {
                    model.record_type(span, &ty)?;
                }
                Ok(ty)
            }
            Spec::Function { .. } => Err(Diagnostic::unsupported("stored function pointers", span)),
        }
    }

    pub(crate) fn label(&self, name: &str, span: Span) -> Result<usize> {
        let id = self
            .scopes
            .iter()
            .rev()
            .find_map(|scope| scope.labels.get(name))
            .copied()
            .ok_or_else(|| {
                Self::error("E201", format!("unknown enclosing label `'{name}`"), span)
            })?;
        if !self
            .frames
            .iter()
            .any(|frame| frame.id == id && frame.owner == self.owner)
        {
            return Err(Self::error(
                "E201",
                format!("label `'{name}` is outside this function"),
                span,
            ));
        }
        Ok(id)
    }

    pub(crate) fn binding_symbol(&mut self, expr: &ast::Expr) -> Result<Option<Value>> {
        let mut form = expr;
        while let ExprKind::Group(value) = &form.kind {
            form = value;
        }
        if let ExprKind::TypeValue(ty) = &form.kind {
            let charge = !super::type_values::transparent_type(form);
            return self.construction_root(expr.span, |checker| {
                if charge {
                    checker.type_work.as_mut().unwrap().logical.charge(1, 0)?;
                }
                let spec = checker.source_spec(ty, charge)?;
                checker
                    .literal_type(spec, ty.span)
                    .map(|ty| Some(Value::Type(ty)))
            });
        }
        let symbol = self.symbol(expr)?;
        if !matches!(form.kind, ExprKind::TypeQuery(_)) {
            let ty = match &symbol {
                Some(Value::Type(ty)) => Some(ty.clone()),
                Some(Value::Foundation(Item::Type(ty))) => Some(Type::Foundation(*ty)),
                _ => None,
            };
            if let Some(ty) = ty {
                self.construction_root(expr.span, |checker| {
                    checker.type_work.as_mut().unwrap().logical.charge(1, 0)?;
                    if matches!(form.kind, ExprKind::Field { .. }) {
                        checker.charge_ancestors(form)?;
                    }
                    checker.type_work.as_mut().unwrap().logical_type(&ty)
                })?;
            }
        }
        Ok(symbol)
    }

    pub(crate) fn symbol(&mut self, expr: &ast::Expr) -> Result<Option<Value>> {
        match &expr.kind {
            ExprKind::Name(name) => Ok(Some(self.value(name, expr.span)?)),
            ExprKind::Group(value) => self.symbol(value),
            ExprKind::Import(name) => {
                if let Some(target) = self.imports.get(&expr.span.start).cloned() {
                    return self.import_module(&target, expr.span).map(Some);
                }
                let module = Module::resolve(name).ok_or_else(|| {
                    Diagnostic::unsupported(format!("module import `@\"{name}\"`"), expr.span)
                })?;
                Ok(Some(Value::Module(module)))
            }
            ExprKind::TypeValue(ty) => Ok(Some(Value::Type(self.type_literal(ty)?))),
            ExprKind::TypeQuery(_) => Ok(Some(Value::Type(self.type_value(expr)?))),
            ExprKind::Field { value, name } => {
                if let ExprKind::Label(label) = &value.kind {
                    let target = self.label(label, expr.span)?;
                    return match name.as_str() {
                        "leave" | "restart" => Ok(Some(Value::Control {
                            target,
                            restart: name == "restart",
                            owner: self.owner,
                        })),
                        _ => Err(Self::error(
                            "E201",
                            format!("unknown label operation `{name}`"),
                            expr.span,
                        )),
                    };
                }
                let symbol = self.symbol(value)?;
                if let Some(Value::Pending(_)) = symbol {
                    return Err(
                        if ["always", "never", "indeterminable"].contains(&name.as_str()) {
                            Diagnostic::unsupported("pending proof scalar projections", expr.span)
                        } else {
                            Self::error(
                                "E201",
                                format!("proof.Result has no field `{name}`"),
                                expr.span,
                            )
                        },
                    );
                }
                if let Some(Value::FileModule { id, ty }) = &symbol {
                    return self.module_member(*id, ty, name, expr.span);
                }
                if let Some(Value::Module(module)) = symbol {
                    let result = match (module, name.as_str()) {
                        (Module::Core, "Type") => {
                            return Err(Diagnostic::unsupported(
                                "first-class core.Type values",
                                expr.span,
                            ));
                        }
                        (Module::Proof, "revision") => Value::Static {
                            value: Constant::Int(1),
                            ty: Type::Int {
                                bits: 32,
                                signed: false,
                            },
                        },
                        (Module::Core, "true") => Value::Constant(Constant::Bool(true)),
                        (Module::Core, "false") => Value::Constant(Constant::Bool(false)),
                        (Module::Core, "null") => Value::Constant(Constant::Null),
                        (Module::Debug, "print") => Value::Print,
                        (Module::Debug, "panic") => Value::Panic,
                        (Module::Core, name) if Self::primitive(name).is_some() => {
                            Value::Type(Self::primitive(name).expect("primitive"))
                        }
                        _ if module.item(name).is_some() => {
                            Value::Foundation(module.item(name).expect("resolved item"))
                        }
                        _ if module.partial() => {
                            return Err(Diagnostic::unsupported(
                                format!("module member `{}.{name}`", module.name()),
                                expr.span,
                            ));
                        }
                        _ => {
                            return Err(Self::error(
                                "E201",
                                format!(
                                    "module `{}` has no supported member `{name}`",
                                    module.name()
                                ),
                                expr.span,
                            ));
                        }
                    };
                    return Ok(Some(result));
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(crate) fn metatype_aliases_resolve_without_runtime_types_or_storage() {
        let program =
            crate::compile("c:@\"core\";alias:c;<Kind>:<alias.Type>;<Again>:<Kind>").unwrap();
        assert!(program.locals.is_empty());
        assert!(program.body.stmts.is_empty());
        let mut checker = Checker::new();
        let span = Span::new(0, 1);
        checker
            .declare("c", Value::Module(Module::Core), span)
            .unwrap();
        let ty = ast::TypeExpr {
            span,
            kind: TypeKind::Name("c.Type".into()),
        };
        assert!(matches!(checker.spec(&ty).unwrap(), Spec::Meta));
        assert_eq!(checker.ty(&ty).unwrap_err().code, "E211");
        assert!(checker.locals.is_empty());
        crate::compile("<Type>:<int32>;n<Type>:4").unwrap();
    }

    #[test]
    pub(crate) fn metatype_storage_and_function_boundaries_remain_closed() {
        for (source, code) in [
            ("c:@\"core\";x<c.Type>:4", "E207"),
            ("c:@\"core\";<R>:<{field<c.Type>}>", "E211"),
            ("c:@\"core\";<R>:<c.Type[1]>", "E211"),
            ("c:@\"core\";<R>:<&c.Type>", "E211"),
            ("c:@\"core\";<R>:<c.Type><null>", "E211"),
            ("c:@\"core\";f<c.Type>:(){-><int32>}", "B001"),
            ("c:@\"core\";f<int32>:(t<c.Type>){->4}", "B001"),
            ("c:@\"core\";<F>:<(c.Type)->int32>", "B001"),
            ("c:@\"core\";value:c.Type", "B001"),
            ("c:@\"core\";value:<c.Type>", "B001"),
        ] {
            let error = crate::compile(source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{source}: {error:?}");
        }
    }
}
