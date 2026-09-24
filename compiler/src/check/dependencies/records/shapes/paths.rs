use super::{Checker, Diagnostic, MAX_DEPTH, Result, ShapeKey, Span, Type};

impl Checker {
    pub(crate) fn shape_field_type<'a>(
        &mut self,
        mut ty: &'a Type,
        key: &ShapeKey,
        span: Span,
    ) -> Result<Option<&'a Type>> {
        if key.fields.len() + key.variants.len() > MAX_DEPTH || !self.flow.spend(key.variants.len())
        {
            return Err(Diagnostic::unsupported(
                "proof result shape path budget exhausted",
                span,
            ));
        }
        let mut variants = key.variants.iter().peekable();
        for at in 0..=key.fields.len() {
            while let Some((index, selected)) = variants.peek() {
                if *index != at {
                    break;
                }
                let Type::Union(members) = ty else {
                    return Ok(None);
                };
                crate::borrow_contract::type_weight(ty, &mut self.flow, span)?;
                crate::borrow_contract::type_weight(selected, &mut self.flow, span)?;
                let Some(member) = members.iter().find(|member| *member == selected) else {
                    return Ok(None);
                };
                ty = member;
                variants.next();
            }
            if at == key.fields.len() {
                break;
            }
            let Some(Type::Record { fields, .. }) = Self::origin_record(ty) else {
                return Ok(None);
            };
            let Some(field) = fields.get(key.fields[at]) else {
                return Ok(None);
            };
            ty = &field.ty;
        }
        Ok(variants.next().is_none().then_some(ty))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::dependencies::{carriers::id, tests::statements};

    #[test]
    pub(crate) fn result_shape_types_follow_exact_variants_and_nullable_paths() {
        let mut checker = Checker::new();
        statements(
            &mut checker,
            "<A>:<{r<&boolean>}>;<B>:<{a<boolean>;r<&int32>}>;<U>:<A><B><null>;x:=false;wide<U>:{->r:&x};outer:{->inner:wide};nullable<{r<&boolean>}><null>:null",
        );
        let ty = checker.locals[id(&checker, "wide")].clone();
        let Type::Union(members) = &ty else { panic!() };
        for member in members
            .iter()
            .filter(|ty| matches!(ty, Type::Record { .. }))
        {
            let Type::Record { fields, .. } = member else {
                panic!()
            };
            let index = fields.iter().position(|field| field.name == "r").unwrap();
            let key = ShapeKey {
                fields: vec![index],
                variants: vec![(0, member.clone())],
            };
            assert_eq!(
                checker
                    .shape_field_type(&ty, &key, Span::new(0, 1))
                    .unwrap(),
                Some(&fields[index].ty)
            );
            let outer = checker.locals[id(&checker, "outer")].clone();
            let key = ShapeKey {
                fields: vec![0, index],
                variants: vec![(1, member.clone())],
            };
            assert_eq!(
                checker
                    .shape_field_type(&outer, &key, Span::new(0, 1))
                    .unwrap(),
                Some(&fields[index].ty)
            );
        }
        let nullable = checker.locals[id(&checker, "nullable")].clone();
        let key = ShapeKey {
            fields: vec![0],
            variants: vec![],
        };
        assert_eq!(
            checker
                .shape_field_type(&nullable, &key, Span::new(0, 1))
                .unwrap(),
            Some(&Type::Reference(Box::new(Type::Bool)))
        );
    }

    #[test]
    pub(crate) fn result_shape_types_reject_stale_variants_and_bound_work() {
        let mut checker = Checker::new();
        statements(
            &mut checker,
            "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;wide<A><B>:{->r:&x}",
        );
        let ty = checker.locals[id(&checker, "wide")].clone();
        let span = Span::new(0, 1);
        for key in [
            ShapeKey {
                fields: vec![0],
                variants: vec![],
            },
            ShapeKey {
                fields: vec![0],
                variants: vec![(0, Type::Bool)],
            },
            ShapeKey {
                fields: vec![],
                variants: vec![(1, Type::Bool)],
            },
        ] {
            assert!(checker.shape_field_type(&ty, &key, span).unwrap().is_none());
        }
        let key = ShapeKey {
            fields: vec![0; 33],
            variants: vec![],
        };
        assert_eq!(
            checker.shape_field_type(&ty, &key, span).unwrap_err().code,
            "B001"
        );
        assert!(!checker.flow.spend(usize::MAX));
        let key = ShapeKey {
            fields: vec![],
            variants: vec![],
        };
        assert!(checker.shape_field_type(&ty, &key, span).is_err());
    }
}
