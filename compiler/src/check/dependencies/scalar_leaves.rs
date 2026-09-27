use super::{
    PointKind,
    edges::{Edge, Port, Route},
};
use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Null,
    Bool,
    Int { bits: u32, signed: bool },
    Float { bits: u32 },
    String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ScalarLeaf {
    pub(crate) owner: usize,
    pub(crate) kind: Kind,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: [Edge; 2],
}

impl Checker {
    pub(crate) fn scalar_leaf(&mut self, value: hir::Expr) -> Result<hir::Expr> {
        if !self.required
            && let Some(id) = self.point
        {
            self.capture_scalar_leaf(id, &value)?;
        }
        Ok(value)
    }

    pub(crate) fn capture_scalar_leaf(
        &mut self,
        id: hir::PointId,
        value: &hir::Expr,
    ) -> Result<()> {
        let invalid = || Diagnostic::unsupported("proof scalar-leaf identity mismatch", value.span);
        if !self
            .flow
            .spend(self.scalar_leaves.len().checked_ilog2().unwrap_or(0) as usize * 2 + 4)
        {
            return Err(Diagnostic::unsupported(
                "proof scalar-leaf budget exhausted",
                value.span,
            ));
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if point.kind != PointKind::Expr
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
        {
            return Err(invalid());
        }
        let kind = match (&value.kind, &value.ty) {
            (hir::ExprKind::Null, hir::Type::Null) => Kind::Null,
            (hir::ExprKind::Bool(_), hir::Type::Bool) => Kind::Bool,
            (hir::ExprKind::Int(_), hir::Type::Int { bits, signed }) => Kind::Int {
                bits: *bits,
                signed: *signed,
            },
            (hir::ExprKind::Float(_), hir::Type::Float { bits }) => Kind::Float { bits: *bits },
            (hir::ExprKind::String(_), hir::Type::String) => Kind::String,
            _ => return Err(invalid()),
        };
        let leaf = ScalarLeaf {
            owner: self.owner,
            kind,
            control: self.control,
            span: value.span,
            edges: [
                Edge::new(Port::Entry(id), Port::Operation(id), Route::Next),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
            ],
        };
        if let Some(prior) = self.scalar_leaves.get(&id) {
            return if *prior == leaf {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(leaf.edges.len()) {
            return Err(Diagnostic::unsupported(
                "proof scalar-leaf budget exhausted",
                value.span,
            ));
        }
        self.scalar_leaf_edges += leaf.edges.len();
        self.scalar_leaves.insert(id, leaf);
        Ok(())
    }
}

#[cfg(test)]
mod stages;

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn expr(source: &str) -> crate::ast::Expr {
        let crate::ast::StmtKind::Expr(expr) =
            crate::parser::parse(source).unwrap().stmts.remove(0).kind
        else {
            panic!()
        };
        expr
    }

    pub(crate) fn setup(source: &str) -> Checker {
        let mut checker = Checker::new();
        for stmt in crate::parser::parse(source).unwrap().stmts {
            checker.stmt(&stmt).unwrap();
        }
        checker
    }

    #[test]
    pub(crate) fn scalar_leaf_capture_keeps_checked_kinds_widths_and_signed_literal_roots() {
        for (source, ty, kind) in [
            (
                "-128",
                hir::Type::Int {
                    bits: 8,
                    signed: true,
                },
                Kind::Int {
                    bits: 8,
                    signed: true,
                },
            ),
            (
                "255",
                hir::Type::Int {
                    bits: 8,
                    signed: false,
                },
                Kind::Int {
                    bits: 8,
                    signed: false,
                },
            ),
            (
                "1.25",
                hir::Type::Float { bits: 32 },
                Kind::Float { bits: 32 },
            ),
            ("true", hir::Type::Bool, Kind::Bool),
            ("null", hir::Type::Null, Kind::Null),
            ("\"é🙂\"", hir::Type::String, Kind::String),
        ] {
            let mut checker = Checker::new();
            let (outer, value) = checker.expr_point(&expr(source), Some(&ty)).unwrap();
            let (&id, leaf) = checker.scalar_leaves.first_key_value().unwrap();
            assert_eq!(checker.scalar_leaves.len(), 1);
            assert_eq!(leaf.kind, kind);
            assert_eq!(value.ty, ty);
            assert_eq!(checker.points[id].parent, Some(outer));
            assert_eq!(checker.coercions[&outer].input, id);
            assert!(checker.unaries.is_empty());
        }
    }

    #[test]
    pub(crate) fn scalar_leaf_capture_follows_resolved_constants_without_group_duplicates() {
        let mut checker =
            setup("core:@\"core\";other:core;truth:core.true;p:@\"proof\";revision:p.revision");
        for (source, kind) in [
            ("core.true", Kind::Bool),
            ("other.false", Kind::Bool),
            ("core.null", Kind::Null),
            (
                "revision",
                Kind::Int {
                    bits: 32,
                    signed: false,
                },
            ),
            (
                "p.revision",
                Kind::Int {
                    bits: 32,
                    signed: false,
                },
            ),
        ] {
            let (id, _) = checker.expr_point(&expr(source), None).unwrap();
            assert_eq!(checker.scalar_leaves[&id].kind, kind);
        }
        let (id, _) = checker.expr_point(&expr("truth"), None).unwrap();
        assert!(!checker.scalar_leaves.contains_key(&id));
        assert!(
            checker
                .local_reads
                .contains_key(&checker.narrowings[&id].input)
        );
        let mut checker = Checker::new();
        let (id, _) = checker.expr_point(&expr("((7))"), None).unwrap();
        assert_eq!(checker.scalar_leaves.len(), 1);
        assert!(!checker.scalar_leaves.contains_key(&id));
        let checker = setup("{true:7;value:true}");
        assert!(
            checker
                .scalar_leaves
                .values()
                .all(|leaf| leaf.kind != Kind::Bool)
        );
        assert_eq!(checker.local_reads.len(), 1);
    }

    #[test]
    pub(crate) fn scalar_leaf_capture_preserves_required_probes_and_literal_errors() {
        let mut checker = Checker::new();
        checker.required = true;
        checker.expr_point(&expr("7"), None).unwrap();
        assert!(checker.scalar_leaves.is_empty());
        checker.required = false;
        checker.integer("7", false, None, Span::default()).unwrap();
        Checker::floating("1.25", None, Span::default()).unwrap();
        assert!(checker.scalar_leaves.is_empty());
        for (source, ty, code) in [
            (
                "256",
                hir::Type::Int {
                    bits: 8,
                    signed: false,
                },
                "E216",
            ),
            (
                "-1",
                hir::Type::Int {
                    bits: 8,
                    signed: false,
                },
                "E222",
            ),
            ("1e999", hir::Type::Float { bits: 64 }, "E216"),
        ] {
            let mut checker = Checker::new();
            assert_eq!(
                checker
                    .expr_point(&expr(source), Some(&ty))
                    .unwrap_err()
                    .code,
                code
            );
            assert!(checker.scalar_leaves.is_empty());
            assert!(checker.point.is_none());
        }
    }
}
