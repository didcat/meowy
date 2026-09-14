use super::{Checker, Constant, Result, Spec, Value};
use crate::ast::{Expr, ExprKind, Span, TypeExpr, TypeKind};
use crate::diagnostic::Diagnostic;
use crate::documentation::{Kind, Model};
use crate::hir::Type;

impl Checker {
    pub(crate) fn doc_stage(&mut self, stage: usize) -> Result<()> {
        let Some(mut model) = self.documentation.take() else {
            return Ok(());
        };
        let ids = model
            .entries
            .iter()
            .enumerate()
            .filter_map(|(id, entry)| (entry.stage == stage && !entry.checked).then_some(id))
            .collect::<Vec<_>>();
        model.spend(model.entries.len() + 1)?;
        for id in ids {
            let entry = &model.entries[id];
            let signature = match entry.kind {
                Kind::Module => "module".into(),
                Kind::Field => entry
                    .type_at
                    .and_then(|at| model.types.get(&at))
                    .cloned()
                    .unwrap_or_default(),
                Kind::Type => {
                    let spec = self
                        .scopes
                        .iter()
                        .rev()
                        .find_map(|scope| scope.types.get(&entry.name))
                        .cloned();
                    match spec {
                        Some(spec) => spec_name(&spec),
                        None => String::new(),
                    }
                }
                _ if entry
                    .type_at
                    .is_some_and(|at| model.types.get(&at).is_some_and(|ty| ty == "core.Type")) =>
                {
                    "core.Type".into()
                }
                _ => {
                    let value = self
                        .scopes
                        .iter()
                        .rev()
                        .find(|scope| scope.doc_values.get(&entry.name) == Some(&entry.span.start))
                        .and_then(|scope| scope.values.get(&entry.name))
                        .or_else(|| {
                            self.scopes
                                .iter()
                                .rev()
                                .find_map(|scope| scope.values.get(&entry.name))
                        })
                        .cloned();
                    value
                        .map(|value| self.doc_value_name(&value))
                        .unwrap_or_default()
                }
            };
            model.spend(signature.len() + 1)?;
            model.entries[id].signature = signature;
            let links = model.entries[id].links.clone();
            for (index, link) in links.iter().enumerate() {
                model.spend(link.target.len() + model.entries.len() + self.scopes.len())?;
                let target =
                    self.doc_resolve(&model, &link.target, link.span)
                        .map_err(|error| {
                            if error.code == "B001" {
                                error
                            } else {
                                Diagnostic::new("E802", error.message, link.span)
                            }
                        })?;
                if model.entries[id].public
                    && target.is_some_and(|target| !model.entries[target].public)
                {
                    return Err(Diagnostic::new(
                        "E802",
                        "public documentation cannot expose a private declaration",
                        link.span,
                    ));
                }
                model.entries[id].links[index].resolved = target;
            }
            model.entries[id].checked = true;
        }
        self.documentation = Some(model);
        Ok(())
    }
    pub(crate) fn doc_value_name(&self, value: &Value) -> String {
        match value {
            Value::Local { ty, .. }
            | Value::Static { ty, .. }
            | Value::Record { ty, .. }
            | Value::Type(ty) => crate::documentation::model::type_name(ty),
            Value::Function { id, params, result } => {
                let result = result.as_ref().or_else(|| {
                    self.functions
                        .get(*id)
                        .and_then(Option::as_ref)
                        .map(|function| &function.result)
                });
                format!(
                    "({})->{}",
                    params
                        .iter()
                        .map(crate::documentation::model::type_name)
                        .collect::<Vec<_>>()
                        .join(","),
                    result
                        .map(crate::documentation::model::type_name)
                        .unwrap_or_else(|| "unresolved".into())
                )
            }
            Value::Module(module) => format!("module {}", module.name()),
            Value::FileModule { .. } => "file module".into(),
            Value::Foundation(item) => format!("intrinsic {}", item.name()),
            Value::Print => "intrinsic debug.print".into(),
            Value::Panic => "intrinsic debug.panic".into(),
            Value::Control { .. } => "scope control".into(),
            Value::Constant(value) => match value {
                Constant::Null => "null",
                Constant::Bool(_) => "boolean",
                Constant::Int(_) => "int32",
                Constant::Float(_) => "float64",
                Constant::String(_) => "string",
            }
            .into(),
        }
    }
    pub(crate) fn doc_resolve(
        &mut self,
        model: &Model,
        target: &str,
        span: Span,
    ) -> Result<Option<usize>> {
        if let Some(name) = target
            .strip_prefix('<')
            .and_then(|name| name.strip_suffix('>'))
        {
            self.spec(&TypeExpr {
                kind: TypeKind::Name(name.into()),
                span,
            })?;
            return Ok(self
                .scopes
                .iter()
                .rev()
                .find_map(|scope| scope.doc_types.get(name))
                .and_then(|start| model.starts.get(start))
                .copied());
        }
        let mut parts = target.split('.');
        let name = parts.next().unwrap_or_default();
        let scope = self
            .scopes
            .iter()
            .rev()
            .find(|scope| scope.values.contains_key(name))
            .ok_or_else(|| {
                Diagnostic::new(
                    "E802",
                    format!("unknown documentation value `{name}`"),
                    span,
                )
            })?;
        let mut value = scope.values[name].clone();
        let mut anchor = scope
            .doc_values
            .get(name)
            .and_then(|start| model.starts.get(start))
            .copied();
        let mut expr = Expr {
            kind: ExprKind::Name(name.into()),
            span,
        };
        for member in parts {
            expr = Expr {
                kind: ExprKind::Field {
                    value: Box::new(expr),
                    name: member.into(),
                },
                span,
            };
            value = match value {
                Value::Module(_) => self.symbol(&expr)?.ok_or_else(|| {
                    Diagnostic::new("E802", "unresolved documentation member", span)
                })?,
                Value::FileModule { id, ty } => {
                    anchor = None;
                    if let Some(value) = self.module_member(id, &ty, member, span)? {
                        value
                    } else {
                        let Type::Record { fields, .. } = ty else {
                            unreachable!()
                        };
                        Value::Type(
                            fields
                                .into_iter()
                                .find(|field| field.name == member)
                                .unwrap()
                                .ty,
                        )
                    }
                }
                Value::Local { ty, .. } | Value::Type(ty) => {
                    let mut ty = &ty;
                    while let Some(inner) = ty.pointee() {
                        ty = inner;
                    }
                    let Type::Record { fields, .. } = ty else {
                        return Err(Diagnostic::new(
                            "E802",
                            "documentation member requires a record or module",
                            span,
                        ));
                    };
                    let field = fields
                        .iter()
                        .find(|field| field.name == member)
                        .ok_or_else(|| {
                            Diagnostic::new(
                                "E802",
                                format!("unknown documentation field `{member}`"),
                                span,
                            )
                        })?;
                    Value::Type(field.ty.clone())
                }
                _ => {
                    return Err(Diagnostic::new(
                        "E802",
                        "documentation target has no members",
                        span,
                    ));
                }
            };
            anchor = anchor.map(|id| model.member(id, member).unwrap_or(id));
        }
        Ok(anchor)
    }
}

pub(crate) fn spec_name(spec: &Spec) -> String {
    match spec {
        Spec::Meta => "core.Type".into(),
        Spec::Descriptor(ty) => ty.name().into(),
        Spec::Data(ty) => crate::documentation::model::type_name(ty),
        Spec::Function { params, result } => format!(
            "({})->{}",
            params
                .iter()
                .map(crate::documentation::model::type_name)
                .collect::<Vec<_>>()
                .join(","),
            crate::documentation::model::type_name(result)
        ),
    }
}
