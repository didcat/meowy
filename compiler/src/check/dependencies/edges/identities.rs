use super::*;
use crate::{
    check::Value,
    foundation::{Item, Module},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindingIdentity {
    Function(crate::hir::FunctionId),
    Foundation,
    Type,
    Static,
    FileModule(crate::hir::LocalId),
    Control {
        target: crate::hir::BlockId,
        owner: usize,
    },
}

impl BindingIdentity {
    pub(crate) fn capture(value: &Value) -> Option<Self> {
        match value {
            Value::Function { id, .. } => Some(Self::Function(*id)),
            Value::Type(_) | Value::Foundation(Item::Type(_)) => Some(Self::Type),
            Value::Static { .. } => Some(Self::Static),
            Value::FileModule { id, .. } => Some(Self::FileModule(*id)),
            Value::Control { target, owner, .. } => Some(Self::Control {
                target: *target,
                owner: *owner,
            }),
            Value::Module(
                Module::Core
                | Module::Debug
                | Module::Memory
                | Module::Strings
                | Module::Proof
                | Module::Bits,
            )
            | Value::Print
            | Value::Panic
            | Value::Foundation(Item::StringCopy | Item::CanCopy | Item::Bits(_)) => {
                Some(Self::Foundation)
            }
            _ => None,
        }
    }
}

impl Checker {
    pub(crate) fn identity_binding_endpoint(
        &mut self,
        id: PointId,
        identity: BindingIdentity,
        span: Span,
    ) -> Result<()> {
        let invalid =
            || Diagnostic::unsupported("proof identity binding endpoint identity mismatch", span);
        let budget =
            || Diagnostic::unsupported("proof identity binding endpoint budget exhausted", span);
        if !self
            .flow
            .spend(self.endpoints.len().checked_ilog2().unwrap_or(0) as usize * 2 + 4)
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if point.kind != PointKind::Stmt
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
        {
            return Err(invalid());
        }
        if let BindingIdentity::Function(target) = identity
            && !self
                .functions
                .get(target)
                .is_some_and(|item| item.as_ref().is_none_or(|function| function.id == target))
        {
            return Err(invalid());
        }
        if let BindingIdentity::FileModule(module) = identity {
            if !self
                .flow
                .spend(self.exports.len().checked_ilog2().unwrap_or(0) as usize + 2)
            {
                return Err(budget());
            }
            if self.locals.get(module).is_none() || !self.exports.contains_key(&module) {
                return Err(invalid());
            }
        }
        if let BindingIdentity::Control { target, owner } = identity {
            if !self.flow.spend(self.frames.len()) {
                return Err(budget());
            }
            if owner != self.owner
                || !self
                    .frames
                    .iter()
                    .any(|frame| frame.id == target && frame.owner == owner)
            {
                return Err(invalid());
            }
        }
        self.publish_declaration_endpoint(id, span, "identity binding")
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod types;

#[cfg(test)]
mod exports;

#[cfg(test)]
mod statics;

#[cfg(test)]
mod controls;

#[cfg(test)]
mod modules;

#[cfg(test)]
mod records;
