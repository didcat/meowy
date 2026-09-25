use super::{Checker, Place, Result, Value};
use crate::ast::{self, ExprKind, Span};
use crate::diagnostic::Diagnostic;
use crate::hir::{self, Type};

impl Checker {
    pub(crate) fn deref_point(
        &mut self,
        value: &ast::Expr,
        span: Span,
    ) -> Result<(hir::PointId, hir::Expr)> {
        let (point, value) = self.expr_point(value, None)?;
        let ty = if value.ty == Type::Never {
            None
        } else {
            Some(
                value
                    .ty
                    .pointee()
                    .ok_or_else(|| {
                        Self::error("E222", "dereference requires a safe reference", span)
                    })?
                    .clone(),
            )
        };
        if let Some(id) = self.point {
            self.deref_operation(id, point, &value, span)?;
        }
        let value = if let Some(ty) = ty {
            hir::Expr {
                ty,
                kind: hir::ExprKind::Deref(Box::new(value)),
                span,
            }
        } else {
            value
        };
        Ok((point, value))
    }

    pub(crate) fn reference_type(&mut self, ty: Type, span: Span) -> Result<Type> {
        if ty.has_exclusive() {
            return Err(Diagnostic::unsupported(
                "references containing exclusive values",
                span,
            ));
        }
        let mut pending = vec![(&ty, 1usize)];
        while let Some((ty, depth)) = pending.pop() {
            if !self.flow.spend(1) {
                return Err(crate::borrow_value::State::budget(span));
            }
            match ty {
                Type::Reference(ty) | Type::Exclusive(ty) => {
                    if depth >= 64 {
                        return Err(Diagnostic::unsupported(
                            "reference nesting budget exhausted",
                            span,
                        ));
                    }
                    pending.push((ty, depth + 1));
                }
                Type::Record { primary, fields } => {
                    for ty in std::iter::once(primary.as_ref())
                        .chain(fields.iter().map(|field| &field.ty))
                    {
                        if pending.len() >= crate::borrow_value::MAX_PARTS || !self.flow.spend(1) {
                            return Err(crate::borrow_value::State::budget(span));
                        }
                        pending.push((ty, depth));
                    }
                }
                Type::Union(members) => {
                    for ty in members {
                        if pending.len() >= crate::borrow_value::MAX_PARTS || !self.flow.spend(1) {
                            return Err(crate::borrow_value::State::budget(span));
                        }
                        pending.push((ty, depth));
                    }
                }
                _ => {}
            }
        }
        Ok(Type::Reference(Box::new(ty)))
    }

    pub(crate) fn exclusive_type(&mut self, ty: Type, span: Span) -> Result<Type> {
        if !matches!(ty, Type::Bool | Type::Int { .. } | Type::Float { .. }) {
            return Err(Diagnostic::unsupported(
                "exclusive references outside ordinary scalar storage",
                span,
            ));
        }
        Ok(Type::Exclusive(Box::new(ty)))
    }

    pub(crate) fn exclusive_borrow(&mut self, expr: &ast::Expr, span: Span) -> Result<hir::Expr> {
        if let ExprKind::Group(value) = &expr.kind {
            return self.exclusive_borrow(value, span);
        }
        if let ExprKind::Unary { op, value } = &expr.kind
            && op == "*"
        {
            return self
                .exclusive_reborrow_point(value, span)
                .map(|(_, value)| value);
        }
        if let Some(value) = self.exclusive_indexed(expr, span)? {
            return Ok(value);
        }
        let (place, ty, _) = self.exclusive_place(expr, span, false)?;
        let ty = self.exclusive_type(ty, span)?;
        Ok(hir::Expr {
            kind: hir::ExprKind::Borrow(place),
            ty,
            span,
        })
    }

    pub(crate) fn exclusive_reborrow_point(
        &mut self,
        value: &ast::Expr,
        span: Span,
    ) -> Result<(hir::PointId, hir::Expr)> {
        let (point, value) = self.expr_point(value, None)?;
        if value.ty == Type::Never {
            return Ok((point, value));
        }
        let Type::Exclusive(ty) = &value.ty else {
            return Err(Self::error(
                "E305",
                "exclusive reborrow requires exclusive access",
                span,
            ));
        };
        let ty = self.exclusive_type(*ty.clone(), span)?;
        let site = self.reborrows;
        self.reborrows += 1;
        Ok((
            point,
            hir::Expr {
                kind: hir::ExprKind::Reborrow {
                    site,
                    value: Box::new(value),
                    fields: Vec::new(),
                },
                ty,
                span,
            },
        ))
    }

    pub(crate) fn exclusive_place(
        &mut self,
        expr: &ast::Expr,
        span: Span,
        indexed: bool,
    ) -> Result<(hir::Place, Type, bool)> {
        let mut root = expr;
        let mut steps = Vec::new();
        loop {
            if !self.flow.spend(1) {
                return Err(Diagnostic::unsupported(
                    "exclusive borrow path budget exhausted",
                    span,
                ));
            }
            match &root.kind {
                ExprKind::Group(value) => root = value,
                ExprKind::Field { value, name } => {
                    if steps.len() == crate::list::MAX_WRITE_PATH {
                        return Err(Diagnostic::unsupported(
                            "exclusive borrow path budget exhausted",
                            span,
                        ));
                    }
                    steps.push((name, root.span));
                    root = value;
                }
                _ => break,
            }
        }
        let ExprKind::Name(name) = &root.kind else {
            return Err(Diagnostic::unsupported(
                "exclusive borrowing outside ordinary named storage",
                span,
            ));
        };
        let Value::Local {
            id, ty, mutable, ..
        } = self.value(name, root.span)?
        else {
            return Err(Diagnostic::unsupported(
                "exclusive borrowing outside local storage",
                span,
            ));
        };
        let alias = self.proofs.aliases.contains_key(&id);
        if !self.places.contains(&id) && !alias {
            return Err(Diagnostic::unsupported(
                "exclusive borrowing of emitted storage",
                span,
            ));
        }
        if !steps.is_empty() {
            let weight = crate::borrow_contract::type_weight(&ty, &mut self.flow, span)?;
            if !self.flow.spend(weight.saturating_mul(2)) {
                return Err(crate::borrow_value::State::budget(span));
            }
            if !matches!(ty, Type::Record { .. }) || ty.has_reference() || !ty.is_copy() {
                return Err(Diagnostic::unsupported(
                    "exclusive field borrowing requires reference-free Copy record storage",
                    span,
                ));
            }
        }
        let mut ty = ty;
        let mut mutable = mutable;
        let mut fields = Vec::new();
        for (name, span) in steps.into_iter().rev() {
            let (index, field, writable) = self.record_field(ty, name, span)?;
            mutable = writable;
            fields.push(index);
            ty = field;
        }
        if indexed {
            let weight = crate::borrow_contract::type_weight(&ty, &mut self.flow, span)?;
            if !self.flow.spend(weight.saturating_mul(2)) {
                return Err(crate::borrow_value::State::budget(span));
            }
            if !matches!(ty, Type::List { .. }) || ty.has_reference() || !ty.is_copy() {
                return Err(Diagnostic::unsupported(
                    "exclusive elements require reference-free Copy list storage",
                    span,
                ));
            }
        } else {
            self.exclusive_type(ty.clone(), span)?;
        }
        if !indexed && !mutable {
            return Err(Self::error(
                "E305",
                "exclusive borrow target is immutable",
                span,
            ));
        }
        if alias {
            self.proofs
                .aliases
                .get_mut(&id)
                .expect("emitted alias")
                .exclusive
                .get_or_insert(span);
        }
        Ok((hir::Place { root: id, fields }, ty, mutable))
    }

    pub(crate) fn address_root<'a>(expr: &'a ast::Expr, fields: &mut Vec<String>) -> &'a ast::Expr {
        match &expr.kind {
            ExprKind::Group(value) => Self::address_root(value, fields),
            ExprKind::Field { value, name } => {
                let root = Self::address_root(value, fields);
                fields.push(name.clone());
                root
            }
            _ => expr,
        }
    }

    pub(crate) fn address_hint(&mut self, expr: &ast::Expr) -> Option<Type> {
        if let Ok((_, ty)) = self.address(expr) {
            return Some(ty);
        }
        match &expr.kind {
            ExprKind::Group(value) => self.address_hint(value),
            ExprKind::Unary { op, value } if op == "*" => match self.hint(value)? {
                Type::Reference(ty) | Type::Exclusive(ty) => Some(*ty),
                _ => None,
            },
            ExprKind::Field { .. } => self.hint(expr),
            ExprKind::Index { value, .. } => {
                let ty = self.address_hint(value).or_else(|| self.hint(value))?;
                let ty = if let Type::Reference(ty) = ty {
                    *ty
                } else {
                    ty
                };
                match ty {
                    Type::List { element, .. } => Some(*element),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub(crate) fn borrowed_point(
        &mut self,
        expr: &ast::Expr,
        span: Span,
    ) -> Result<(hir::PointId, hir::Expr)> {
        self.with_continuation(expr.span, "borrowed expression", |checker| {
            checker.with_point_id(super::PointKind::Expr, expr.span, |checker| {
                checker.borrowed(expr, span)
            })
        })
    }

    pub(crate) fn borrowed(&mut self, expr: &ast::Expr, span: Span) -> Result<hir::Expr> {
        if !self.imports.is_empty() {
            let mut root = expr;
            loop {
                if !self.flow.spend(1) {
                    return Err(Diagnostic::unsupported(
                        "file-module reference path budget exhausted",
                        span,
                    ));
                }
                match &root.kind {
                    ExprKind::Group(value)
                    | ExprKind::Field { value, .. }
                    | ExprKind::Index { value, .. } => root = value,
                    _ => break,
                }
            }
            if matches!(root.kind, ExprKind::Name(_) | ExprKind::Import(_))
                && matches!(self.symbol(root)?, Some(Value::FileModule { .. }))
            {
                return Err(Diagnostic::unsupported(
                    "references to file-module storage",
                    span,
                ));
            }
        }
        match &expr.kind {
            ExprKind::Group(value) => return self.borrowed(value, span),
            ExprKind::Index { value, index } => return self.element_borrow(value, index, span),
            _ => {}
        }
        let error = match self.address(expr) {
            Ok((place, ty)) => {
                if let Some(alias) = self.proofs.aliases.get_mut(&place.root) {
                    alias.borrowed.get_or_insert(span);
                }
                return Ok(hir::Expr {
                    kind: hir::ExprKind::Borrow(place),
                    ty: self.reference_type(ty, span)?,
                    span,
                });
            }
            Err(error) => error,
        };
        let mut names = Vec::new();
        let root = Self::address_root(expr, &mut names);
        let mut value = if matches!(root.kind, ExprKind::Index { .. }) {
            self.borrowed_point(root, root.span)?.1
        } else if let ExprKind::Unary { op, value } = &root.kind
            && op == "*"
        {
            self.expr(value, None)?
        } else {
            let value = self.expr(root, None)?;
            if names.is_empty() {
                return self.temporary_borrow(value, span);
            }
            if !matches!(value.ty, Type::Reference(_) | Type::Exclusive(_))
                && self.address(root).is_err()
            {
                self.temporary_borrow(value, root.span)?
            } else {
                value
            }
        };
        if value.ty == Type::Never {
            return Ok(value);
        }
        let mut names = names.into_iter().peekable();
        while !matches!(value.ty, Type::Reference(_) | Type::Exclusive(_)) {
            let Some(name) = names.next() else {
                return Err(error);
            };
            let Type::Record { fields, .. } = &value.ty else {
                return Err(error);
            };
            let (index, ty) = fields
                .iter()
                .enumerate()
                .find(|(_, field)| field.name == name)
                .map(|(index, field)| (index, field.ty.clone()))
                .ok_or_else(|| {
                    Self::error("E201", format!("unknown record field `{name}`"), span)
                })?;
            value = self.narrow(hir::Expr {
                kind: hir::ExprKind::Field {
                    value: Box::new(value),
                    index,
                },
                ty,
                span: expr.span,
            })?;
            if names.peek().is_none() {
                return Err(error);
            }
        }
        loop {
            let (Type::Reference(target) | Type::Exclusive(target)) = &value.ty else {
                unreachable!()
            };
            let mut ty = target.as_ref();
            let mut path = Vec::new();
            while let Some(name) = names.next() {
                let Type::Record { fields, .. } = ty else {
                    return Err(Diagnostic::unsupported(
                        "reborrow projection outside concrete record storage",
                        span,
                    ));
                };
                if !self.flow.spend(fields.len() + path.len() + 1) {
                    return Err(crate::borrow_value::State::budget(span));
                }
                let (index, field) = fields
                    .iter()
                    .enumerate()
                    .find(|(_, field)| field.name == name)
                    .ok_or_else(|| {
                        Self::error("E201", format!("unknown record field `{name}`"), span)
                    })?;
                path.push(index);
                ty = &field.ty;
                if matches!(ty, Type::Reference(_) | Type::Exclusive(_)) && names.peek().is_some() {
                    break;
                }
            }
            crate::borrow_contract::type_weight(ty, &mut self.flow, span)?;
            if names.peek().is_none() {
                let ty = self.reference_type(ty.clone(), span)?;
                let site = self.reborrows;
                self.reborrows += 1;
                return Ok(hir::Expr {
                    kind: hir::ExprKind::Reborrow {
                        site,
                        value: Box::new(value),
                        fields: path,
                    },
                    ty,
                    span,
                });
            }
            crate::borrow_contract::type_weight(target, &mut self.flow, span)?;
            value = hir::Expr {
                ty: *target.clone(),
                kind: hir::ExprKind::Deref(Box::new(value)),
                span,
            };
            for index in path {
                let Type::Record { fields, .. } = &value.ty else {
                    unreachable!()
                };
                let ty = &fields[index].ty;
                crate::borrow_contract::type_weight(ty, &mut self.flow, span)?;
                value = hir::Expr {
                    ty: ty.clone(),
                    kind: hir::ExprKind::Field {
                        value: Box::new(value),
                        index,
                    },
                    span,
                };
            }
        }
    }

    pub(crate) fn address(&self, expr: &ast::Expr) -> Result<(hir::Place, Type)> {
        self.address_storage(expr)
    }

    pub(self) fn address_storage(&self, expr: &ast::Expr) -> Result<(hir::Place, Type)> {
        match &expr.kind {
            ExprKind::Group(value) => self.address_storage(value),
            ExprKind::Name(name) => {
                let Value::Local { id, ty, .. } = self.value(name, expr.span)? else {
                    return Err(Diagnostic::unsupported(
                        "borrowing temporary or intrinsic values",
                        expr.span,
                    ));
                };
                if !self.places.contains(&id) && !self.proofs.aliases.contains_key(&id) {
                    return Err(Diagnostic::unsupported(
                        "borrowing emitted storage",
                        expr.span,
                    ));
                }
                Ok((
                    hir::Place {
                        root: id,
                        fields: Vec::new(),
                    },
                    ty,
                ))
            }
            ExprKind::Field { value, name } => {
                let (mut place, ty) = self.address_storage(value)?;
                let Type::Record { fields, .. } = ty else {
                    return Err(Diagnostic::unsupported(
                        "borrowing fields outside concrete record storage",
                        expr.span,
                    ));
                };
                let (index, field) = fields
                    .into_iter()
                    .enumerate()
                    .find(|(_, field)| &field.name == name)
                    .ok_or_else(|| {
                        Self::error("E201", format!("unknown record field `{name}`"), expr.span)
                    })?;
                place.fields.push(index);
                Ok((place, field.ty))
            }
            _ => Err(Diagnostic::unsupported(
                "borrowing temporary or projected storage",
                expr.span,
            )),
        }
    }

    pub(crate) fn ast_place(&self, expr: &ast::Expr) -> Option<Place> {
        match &expr.kind {
            ExprKind::Name(name) => match self.value(name, expr.span).ok()? {
                Value::Local { id, .. } => Some((id, Vec::new())),
                _ => None,
            },
            ExprKind::Group(value) | ExprKind::Ascribe { value, .. } => self.ast_place(value),
            ExprKind::Field { value, name } => {
                let mut place = self.ast_place(value)?;
                place.1.push(name.clone());
                Some(place)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod dereference;

#[cfg(test)]
mod reborrow;
