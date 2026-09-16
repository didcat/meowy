use super::{Checker, Result, Spec, Value};
use crate::ast::{self, ExprKind, Span};
use crate::diagnostic::Diagnostic;
use crate::hir::{self, Type};
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct Module {
    pub(crate) block: usize,
    pub(crate) depth: usize,
    pub(crate) values: BTreeMap<String, Value>,
    pub(crate) types: BTreeMap<String, Spec>,
    pub(crate) inputs: BTreeMap<String, Input>,
    pub(crate) primary: Option<(hir::EmitId, Primary)>,
}

#[derive(Clone, Debug)]
pub(crate) enum Primary {
    Int(super::inputs::Input),
    Bool(super::inputs::Input<bool>),
}

impl Primary {
    pub(crate) fn forward(&mut self) {
        match self {
            Self::Int(input) => input.work = input.work.saturating_add(2),
            Self::Bool(input) => input.work = input.work.saturating_add(2),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Input {
    pub(crate) id: usize,
    pub(crate) path: Vec<usize>,
    pub(crate) work: usize,
}

impl Checker {
    pub(crate) fn export_type_value(
        &mut self,
        label: Option<&str>,
        name: Option<&str>,
        annotation: Option<&ast::TypeExpr>,
        mutable: bool,
        value: &ast::Expr,
        span: Span,
    ) -> Result<bool> {
        let (Some(name), Some(annotation)) = (name, annotation) else {
            return Ok(false);
        };
        if matches!(value.kind, ExprKind::Function { .. }) || !self.meta_annotation(annotation)? {
            return Ok(false);
        }
        if label.is_some()
            || self.owner != 0
            || self.scopes.len() != self.module.depth
            || self
                .frames
                .last()
                .is_none_or(|frame| frame.id != self.module.block)
        {
            return Err(Diagnostic::unsupported(
                "non-top-level type-value exports",
                span,
            ));
        }
        if mutable {
            return Err(Diagnostic::unsupported("mutable type-value exports", span));
        }
        if !self.flow.spend(name.len() + self.module.values.len() + 1) {
            return Err(Diagnostic::unsupported(
                "module export budget exhausted",
                span,
            ));
        }
        if self.module.values.contains_key(name)
            || self
                .frames
                .last()
                .unwrap()
                .slots
                .contains_key(&Some(name.into()))
        {
            return Err(Self::error(
                "E205",
                format!("module export `{name}` is already emitted"),
                span,
            ));
        }
        let value = self.meta_binding(value, annotation)?;
        self.declare(name, value.clone(), span)?;
        self.module.values.insert(name.into(), value);
        Ok(true)
    }

    pub(crate) fn module_value(
        &mut self,
        value: &ast::Expr,
        expected: Option<&Type>,
    ) -> Result<(hir::Expr, Module)> {
        if !matches!(value.kind, ExprKind::Block(_)) {
            return Err(Diagnostic::unsupported(
                "missing file-module initializer",
                value.span,
            ));
        }
        let saved = std::mem::replace(
            &mut self.module,
            Module {
                block: self.block,
                depth: self.scopes.len() + 1,
                values: BTreeMap::new(),
                types: BTreeMap::new(),
                inputs: BTreeMap::new(),
                primary: None,
            },
        );
        let docs = self.file_docs.remove(&value.span.start);
        let saved_docs = std::mem::replace(&mut self.documentation, docs);
        let result = self.expr(value, expected);
        let module = std::mem::replace(&mut self.module, saved);
        let docs = std::mem::replace(&mut self.documentation, saved_docs);
        let result = result?;
        if let Some(mut model) = docs {
            model.finish()?;
        }
        Ok((result, module))
    }

    pub(crate) fn input_export(&self, target: usize) -> bool {
        target == self.module.block
            && self.owner == 0
            && self.scopes.len() == self.module.depth
            && self.reach != crate::flow::FALSE
    }

    pub(crate) fn primary_input(&mut self, stmt: &hir::Stmt) {
        let hir::Stmt::Emit {
            id,
            target,
            field: None,
            value,
        } = stmt
        else {
            return;
        };
        if !self.input_export(*target) {
            return;
        }
        let input = match value.ty {
            Type::Int { .. } => self.integer_input(value, &value.ty).map(Primary::Int),
            Type::Bool => self.boolean_input(value, &value.ty).map(Primary::Bool),
            _ => None,
        };
        if let Some(input) = input {
            self.module.primary = Some((*id, input));
        }
    }

    pub(crate) fn export_input(&mut self, target: usize, name: &str, id: usize, value: &hir::Expr) {
        if !self.input_export(target) {
            return;
        }
        if let Some(input) = self.integer_input(value, &value.ty) {
            self.inputs.insert(id, input);
            self.module.inputs.insert(
                name.into(),
                Input {
                    id,
                    path: Vec::new(),
                    work: 0,
                },
            );
        }
        if let Some(input) = self.boolean_input(value, &value.ty) {
            self.bool_inputs.insert(id, input);
            self.module.inputs.insert(
                name.into(),
                Input {
                    id,
                    path: Vec::new(),
                    work: 0,
                },
            );
        }
        if let Some(input) = self.record_input(value, &value.ty) {
            self.record_inputs.insert(id, input);
            self.module.inputs.insert(
                name.into(),
                Input {
                    id,
                    path: Vec::new(),
                    work: 0,
                },
            );
        }
    }

    pub(crate) fn composed_inputs(&mut self, target: usize, stmts: &[hir::Stmt]) -> Result<()> {
        if !self.input_export(target) {
            return Ok(());
        }
        let Some(hir::Stmt::Bind { id, value }) = stmts.first() else {
            return Ok(());
        };
        let source = match value.kind {
            hir::ExprKind::Local(id) => self.exports.get(&id),
            _ => None,
        };
        let Some(source) = source else {
            return self.composed_record_inputs(*id, value);
        };
        for stmt in stmts {
            let hir::Stmt::Emit {
                id, field, value, ..
            } = stmt
            else {
                continue;
            };
            if !self
                .flow
                .spend(field.as_ref().map_or(0, String::len) + source.inputs.len() + 1)
            {
                return Err(Diagnostic::unsupported(
                    "module input composition budget exhausted",
                    value.span,
                ));
            }
            if let Some(name) = field {
                if let Some(input) = source.inputs.get(name) {
                    let mut input = input.clone();
                    input.work = input.work.saturating_add(2);
                    self.module.inputs.insert(name.clone(), input);
                }
            } else if matches!(value.ty, Type::Int { .. } | Type::Bool)
                && let Some((_, input)) = &source.primary
            {
                let mut input = input.clone();
                input.forward();
                self.module.primary = Some((*id, input));
            }
        }
        Ok(())
    }

    pub(crate) fn composed_record_inputs(&mut self, id: usize, value: &hir::Expr) -> Result<()> {
        let Some(record) = self.record_input(value, &value.ty) else {
            return Ok(());
        };
        let Type::Record { fields, .. } = &value.ty else {
            return Ok(());
        };
        self.record_inputs.insert(id, record);
        for (index, field) in fields.iter().enumerate() {
            if !self.flow.spend(field.name.len() + fields.len() + 1) {
                return Err(Diagnostic::unsupported(
                    "record input composition budget exhausted",
                    value.span,
                ));
            }
            self.module.inputs.insert(
                field.name.clone(),
                Input {
                    id,
                    path: vec![index],
                    work: 0,
                },
            );
        }
        Ok(())
    }

    pub(crate) fn export_function(
        &mut self,
        label: Option<&str>,
        name: Option<&str>,
        annotation: Option<&ast::TypeExpr>,
        mutable: bool,
        value: &ast::Expr,
        span: Span,
    ) -> Result<bool> {
        if label.is_some()
            || self.owner != 0
            || self
                .frames
                .last()
                .is_none_or(|frame| frame.id != self.module.block)
        {
            return Ok(false);
        }
        let Some(name) = name else { return Ok(false) };
        if !self.flow.spend(name.len() + self.module.values.len() + 1) {
            return Err(Diagnostic::unsupported(
                "module export budget exhausted",
                span,
            ));
        }
        let function = if matches!(value.kind, ExprKind::Function { .. }) {
            None
        } else {
            match self.symbol(value)? {
                Some(value @ Value::Function { .. }) => Some(value),
                _ => return Ok(false),
            }
        };
        if self.module.values.contains_key(name) {
            return Err(Self::error(
                "E205",
                format!("module export `{name}` is already emitted"),
                span,
            ));
        }
        if self.scopes.len() != self.module.depth {
            return Err(Diagnostic::unsupported(
                "conditional function exports",
                span,
            ));
        }
        if self
            .frames
            .last()
            .unwrap()
            .slots
            .contains_key(&Some(name.into()))
        {
            return Err(Self::error(
                "E205",
                format!("module export `{name}` is already emitted"),
                span,
            ));
        }
        if mutable {
            return Err(Diagnostic::unsupported("mutable function exports", span));
        }
        let annotation = annotation.ok_or_else(|| {
            Self::error(
                "E214",
                "exported function requires an explicit signature",
                span,
            )
        })?;
        if let Some(function) = function {
            let signature = self.spec(annotation)?;
            if !matches!((&signature, &function),
                (Spec::Function {params, result}, Value::Function {params: actual, result: Some(found), ..})
                if params == actual && result == found)
            {
                return Err(Self::error(
                    "E207",
                    "exported signature does not match the function",
                    span,
                ));
            }
            self.declare(name, function, span)?;
        } else {
            let ExprKind::Function { params, body } = &value.kind else {
                unreachable!()
            };
            self.declare_function(name, Some(annotation), params, body, span)?;
        }
        self.module
            .values
            .insert(name.into(), self.value(name, span)?);
        Ok(true)
    }

    pub(crate) fn declare_type(
        &mut self,
        name: &str,
        ty: &ast::TypeExpr,
        exported: bool,
        span: Span,
    ) -> Result<()> {
        self.construction_root(ty.span, |checker| {
            checker.declare_source_type(name, ty, exported, span, true)
        })
    }

    pub(crate) fn declare_source_type(
        &mut self,
        name: &str,
        ty: &ast::TypeExpr,
        exported: bool,
        span: Span,
        charge: bool,
    ) -> Result<()> {
        if exported
            && (self.owner != 0
                || self.scopes.len() != self.module.depth
                || self
                    .frames
                    .last()
                    .is_none_or(|frame| frame.id != self.module.block))
        {
            return Err(Diagnostic::unsupported("non-top-level type exports", span));
        }
        if exported && !self.flow.spend(name.len() + self.module.types.len() + 1) {
            return Err(Diagnostic::unsupported(
                "module type export budget exhausted",
                span,
            ));
        }
        let spec = self.source_spec(ty, charge)?;
        if exported {
            Self::charge_spec(&spec, &mut self.flow, span)?;
        }
        let scope = self.scopes.last_mut().expect("scope");
        if scope.types.contains_key(name) {
            return Err(Self::error(
                "E203",
                format!("type `{name}` is already declared in this scope"),
                span,
            ));
        }
        if exported {
            self.module.types.insert(name.into(), spec.clone());
        }
        scope.types.insert(name.into(), spec);
        if self.documentation.is_some() {
            scope.doc_types.insert(name.into(), span.start);
        }
        Ok(())
    }

    pub(crate) fn charge_spec(spec: &Spec, flow: &mut crate::flow::Flow, span: Span) -> Result<()> {
        match spec {
            Spec::Meta | Spec::Descriptor(_) => {
                if !flow.spend(1) {
                    return Err(Diagnostic::unsupported(
                        "module type export budget exhausted",
                        span,
                    ));
                }
            }
            Spec::Data(ty) => {
                crate::borrow_contract::type_weight(ty, flow, span)?;
            }
            Spec::Function { params, result } => {
                for ty in params.iter().chain(std::iter::once(result)) {
                    crate::borrow_contract::type_weight(ty, flow, span)?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn module_type(&mut self, id: usize, name: &str, span: Span) -> Result<Spec> {
        if !self.flow.spend(name.len() + self.exports.len() + 1) {
            return Err(Diagnostic::unsupported(
                "module type lookup budget exhausted",
                span,
            ));
        }
        let spec = self
            .exports
            .get(&id)
            .and_then(|module| module.types.get(name))
            .ok_or_else(|| {
                Self::error(
                    "E202",
                    format!("module has no exported type `{name}`"),
                    span,
                )
            })?;
        Self::charge_spec(spec, &mut self.flow, span)?;
        Ok(spec.clone())
    }

    pub(crate) fn import_module(&mut self, target: &str, span: Span) -> Result<Value> {
        if !self
            .flow
            .spend(self.scopes.len().saturating_mul(target.len() + 1) + self.exports.len() + 1)
        {
            return Err(Diagnostic::unsupported(
                "module identity lookup budget exhausted",
                span,
            ));
        }
        let value = self
            .scopes
            .iter()
            .rev()
            .find_map(|scope| scope.values.get(target));
        let Some(Value::Local { id, ty, .. }) = value else {
            return Err(Diagnostic::unsupported(
                "missing file-module identity",
                span,
            ));
        };
        if !self.exports.contains_key(id) {
            return Err(Diagnostic::unsupported(
                "unregistered file-module identity",
                span,
            ));
        }
        crate::borrow_contract::type_weight(ty, &mut self.flow, span)?;
        Ok(Value::FileModule {
            id: *id,
            ty: ty.clone(),
        })
    }

    pub(crate) fn module_member(
        &mut self,
        id: usize,
        ty: &Type,
        name: &str,
        span: Span,
    ) -> Result<Option<Value>> {
        if !self.flow.spend(name.len() + self.exports.len() + 1) {
            return Err(Diagnostic::unsupported(
                "module member lookup budget exhausted",
                span,
            ));
        }
        if let Some(value) = self
            .exports
            .get(&id)
            .and_then(|module| module.values.get(name))
        {
            return Ok(Some(value.clone()));
        }
        if let Type::Record { fields, .. } = ty {
            if !self.flow.spend(fields.len().saturating_mul(name.len() + 1)) {
                return Err(Diagnostic::unsupported(
                    "module member lookup budget exhausted",
                    span,
                ));
            }
            if fields.iter().any(|field| field.name == name) {
                return Ok(None);
            }
        }
        Err(Self::error(
            "E201",
            format!("module has no exported value `{name}`"),
            span,
        ))
    }
}

#[cfg(test)]
mod tests;
