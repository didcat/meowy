use super::*;
use crate::check::dependencies::{ScalarKind, bodies::Scalar};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Types {
    pub(crate) source: Scalar,
    pub(crate) target: Scalar,
}

impl Types {
    pub(crate) fn valid(self) -> bool {
        !self.source.nullable
            && self.source.shape().is_some()
            && self.target.nullable
            && self.target.shape().is_some()
            && (self.source.kind == ScalarKind::Null || self.source.kind == self.target.kind)
    }
}

impl Coercion {
    pub(crate) fn scalars(value: &hir::Expr) -> Option<Types> {
        let hir::ExprKind::Coerce { value: input } = &value.kind else {
            return None;
        };
        let types = Types {
            source: Scalar::of(&input.ty)?,
            target: Scalar::of(&value.ty)?,
        };
        types.valid().then_some(types)
    }

    pub(crate) fn valid_scalars(kind: Kind, primary: bool, scalar: Option<Types>) -> bool {
        scalar.is_none_or(|types| kind == Kind::Convert && !primary && types.valid())
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{expr, prepare, record};
    use super::*;

    #[test]
    pub(crate) fn scalar_coercions_capture_exact_types_in_expected_and_composed_contexts() {
        for name in [
            "boolean", "int8", "uint8", "int16", "uint16", "int32", "uint32", "int64", "uint64",
        ] {
            for null in [false, true] {
                let literal = if name == "boolean" { "true" } else { "7" };
                let mut checker = prepare(&format!("n<{name}>:{literal};<Choice>:<{name}><null>"));
                let ty = checker.locals[0].clone();
                let expected = hir::Type::union(vec![ty.clone(), hir::Type::Null]);
                let source = if null { "null" } else { "n" };
                for composed in [false, true] {
                    let (id, value) = if composed {
                        checker.composed_point(&expr(source), record(), Some(&expected))
                    } else {
                        checker.operand_coercion(&expr(source), &expected, |checker| {
                            checker.expression_value(&expr(source), Some(&expected))
                        })
                    }
                    .unwrap();
                    let scalar = checker.coercions[&id].scalar.unwrap();
                    assert_eq!(
                        scalar.source,
                        Scalar::of(if null { &hir::Type::Null } else { &ty }).unwrap()
                    );
                    assert_eq!(scalar.target, Scalar::of(&expected).unwrap());
                    assert_eq!(Coercion::scalars(&value), Some(scalar));
                    assert!(scalar.valid());
                }
            }
        }
    }

    #[test]
    pub(crate) fn scalar_coercions_exclude_opaque_widening_projection_and_unchanged_types() {
        let source = "n:7;a<int32>:n;b<boolean><int32>:n;s<string><null>:null;r:{->n;->tag:true};v<int32><null>:r;f<float64><null>:1.5";
        let checker = prepare(source);
        assert!(checker.coercions.values().all(|op| op.scalar.is_none()));
        let int = Scalar::of(&hir::Type::Int {
            bits: 32,
            signed: true,
        })
        .unwrap();
        for types in [
            Types {
                source: int,
                target: int,
            },
            Types {
                source: Scalar {
                    nullable: true,
                    ..int
                },
                target: Scalar {
                    nullable: true,
                    ..int
                },
            },
            Types {
                source: int,
                target: Scalar {
                    kind: ScalarKind::Bool,
                    nullable: true,
                },
            },
            Types {
                source: int,
                target: Scalar {
                    kind: ScalarKind::Int {
                        bits: 32,
                        signed: false,
                    },
                    nullable: true,
                },
            },
        ] {
            assert!(!types.valid());
        }
    }

    #[test]
    pub(crate) fn scalar_coercions_replay_exact_descriptors_and_reject_drift_atomically() {
        let mut checker = prepare("n:7;v<int32><null>:n");
        let (&id, op) = checker
            .coercions
            .iter()
            .find(|(_, op)| op.scalar.is_some())
            .unwrap();
        let op = op.clone();
        let saved = format!("{:?}", checker.coercions);
        let counts = checker.coercion_edges;
        checker
            .typed_coercion_stages(
                (id, op.input),
                op.kind,
                (false, None, None),
                op.scalar,
                op.span,
            )
            .unwrap();
        let mut changed = op.scalar.unwrap();
        changed.source.kind = ScalarKind::Null;
        for scalar in [None, Some(changed)] {
            assert!(
                checker
                    .typed_coercion_stages(
                        (id, op.input),
                        op.kind,
                        (false, None, None),
                        scalar,
                        op.span
                    )
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
            assert_eq!(format!("{:?}", checker.coercions), saved);
            assert_eq!(checker.coercion_edges, counts);
        }
        for kind in [Kind::Forward, Kind::Stopped] {
            assert!(!Coercion::valid_scalars(kind, false, op.scalar));
        }
        assert!(!Coercion::valid_scalars(Kind::Convert, true, op.scalar));
    }
}
