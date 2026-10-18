use super::*;
use crate::hir::Type;

impl Domain {
    pub(in super::super) fn declared(
        ty: &Type,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Option<Self>> {
        if !flow.spend(1) {
            return Err(budget(span));
        }
        let ty = match ty {
            Type::Never => return Ok(Some(Self::BOTTOM)),
            Type::Null => return Ok(Some(Self::NULL)),
            Type::Union(types) if types.len() == 2 => {
                let ty = match (&types[0], &types[1]) {
                    (Type::Null, ty) | (ty, Type::Null) => ty,
                    _ => return Ok(None),
                };
                return Self::plain(ty, flow, span)?
                    .map(|value| value.join(&Self::NULL, flow, span))
                    .transpose();
            }
            ty => ty,
        };
        Self::plain(ty, flow, span)
    }

    pub(self) fn plain(ty: &Type, flow: &mut Flow, span: Span) -> Result<Option<Self>> {
        if !flow.spend(1) {
            return Err(budget(span));
        }
        let values = match ty {
            Type::Bool => Some(Values::Bool(3)),
            Type::Int { bits, signed } if matches!(bits, 8 | 16 | 32 | 64) => {
                let (low, high) = if *signed {
                    (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1)
                } else {
                    (0, (1i128 << bits) - 1)
                };
                Integers::interval(low, high, flow, span)?.map(Values::Int)
            }
            _ => return Ok(None),
        };
        Ok(Some(Self {
            values,
            null: false,
        }))
    }
}
