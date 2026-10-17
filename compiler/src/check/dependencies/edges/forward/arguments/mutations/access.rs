use crate::ast::Span;
use crate::check::dependencies::ScalarKind;
use crate::diagnostic::Diagnostic;
use crate::flow::Flow;
use crate::hir::Type;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super::super) enum Access {
    Value,
    Shared(ScalarKind),
    Exclusive(ScalarKind),
    Unknown,
}

pub(super) fn classify(ty: &Type, flow: &mut Flow, span: Span) -> Result<Access, Diagnostic> {
    let weight = crate::borrow_contract::type_weight(ty, flow, span)?;
    if !flow.spend(weight.saturating_add(1)) {
        return Err(Diagnostic::unsupported(
            "proof declared-access budget exhausted",
            span,
        ));
    }
    match ty {
        Type::Reference(ty) => {
            return Ok(ScalarKind::of(ty).map_or(Access::Unknown, Access::Shared));
        }
        Type::Exclusive(ty) => {
            return Ok(match ScalarKind::of(ty) {
                Some(
                    kind @ (ScalarKind::Bool | ScalarKind::Int { .. } | ScalarKind::Float { .. }),
                ) => Access::Exclusive(kind),
                _ => Access::Unknown,
            });
        }
        _ => {}
    }
    let mut pending = vec![ty];
    while let Some(ty) = pending.pop() {
        if ScalarKind::of(ty).is_some() || matches!(ty, Type::Never) {
            continue;
        }
        match ty {
            Type::Record { primary, fields } => {
                pending.push(primary);
                pending.extend(fields.iter().map(|field| &field.ty));
            }
            Type::List { element, capacity } if *capacity <= crate::list::MAX_CAPACITY => {
                pending.push(element)
            }
            Type::Union(members) if members.len() >= 2 => pending.extend(members),
            _ => return Ok(Access::Unknown),
        }
    }
    Ok(Access::Value)
}

#[cfg(test)]
mod tests;
