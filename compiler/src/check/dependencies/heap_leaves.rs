use super::PointKind;
use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HeapLeaf {
    pub(crate) owner: usize,
    pub(crate) ty: hir::FoundationType,
    pub(crate) control: bool,
    pub(crate) span: Span,
}

impl Checker {
    pub(crate) fn heap_leaf(&mut self, value: hir::Expr) -> Result<hir::Expr> {
        if !self.required
            && let Some(id) = self.point
        {
            self.capture_heap_leaf(id, &value)?;
        }
        Ok(value)
    }

    pub(crate) fn capture_heap_leaf(&mut self, id: hir::PointId, value: &hir::Expr) -> Result<()> {
        let invalid = || Diagnostic::unsupported("proof heap-leaf identity mismatch", value.span);
        if !self
            .flow
            .spend(self.heap_leaves.len().checked_ilog2().unwrap_or(0) as usize * 2 + 4)
        {
            return Err(Diagnostic::unsupported(
                "proof heap-leaf budget exhausted",
                value.span,
            ));
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if point.kind != PointKind::Expr
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || !matches!(value.kind, hir::ExprKind::Heap)
            || value.ty != hir::Type::Foundation(hir::FoundationType::Allocator)
        {
            return Err(invalid());
        }
        let leaf = HeapLeaf {
            owner: self.owner,
            ty: hir::FoundationType::Allocator,
            control: self.control,
            span: value.span,
        };
        if let Some(prior) = self.heap_leaves.get(&id) {
            return if *prior == leaf {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        self.heap_leaves.insert(id, leaf);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::Value;

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
    pub(crate) fn heap_leaf_capture_preserves_resolved_identity_and_group_roots() {
        let mut checker = setup("m:@\"memory\";alias:m");
        checker
            .declare(
                "resolved",
                Value::Foundation(crate::foundation::Item::Heap),
                Span::default(),
            )
            .unwrap();
        for source in ["m.heap", "alias.heap", "resolved"] {
            let expr = expr(source);
            let (id, value) = checker.expr_point(&expr, None).unwrap();
            assert_eq!(checker.heap_leaves[&id].ty, hir::FoundationType::Allocator);
            assert_eq!(checker.heap_leaves[&id].span, expr.span);
            assert!(matches!(value.kind, hir::ExprKind::Heap));
            assert_eq!(
                value.ty,
                hir::Type::Foundation(hir::FoundationType::Allocator)
            );
        }
        let before = checker.heap_leaves.len();
        let (id, _) = checker.expr_point(&expr("((m.heap))"), None).unwrap();
        assert_eq!(checker.heap_leaves.len(), before + 1);
        assert!(!checker.heap_leaves.contains_key(&id));
        assert!(checker.scalar_leaves.is_empty());
    }

    #[test]
    pub(crate) fn heap_leaf_capture_preserves_local_cells_and_shadowing() {
        let source = "m:@\"memory\";alias:m;a:m.heap;b:a;{m:{->heap:7};x:m.heap};c:alias.heap";
        crate::compile(source).unwrap();
        let checker = setup(source);
        assert_eq!(checker.heap_leaves.len(), 2);
        assert!(
            checker
                .local_reads
                .values()
                .any(|read| checker.locals[read.local]
                    == hir::Type::Foundation(hir::FoundationType::Allocator))
        );
        assert_eq!(checker.scalar_leaves.len(), 1);
        assert!(checker.fields.values().any(|field| !field.load));
    }

    #[test]
    pub(crate) fn heap_leaf_capture_keeps_required_paths_and_nominal_capability_errors() {
        let mut checker = setup("m:@\"memory\"");
        checker.required = true;
        checker.expr_point(&expr("m.heap"), None).unwrap();
        assert!(checker.heap_leaves.is_empty());
        for (source, code) in [
            ("m:@\"memory\";a<m.Allocator>:{->n:1}", "E207"),
            ("m:@\"memory\";m.missing", "B001"),
            ("m:@\"memory\";d:@\"debug\";d.print(m.heap)", "B001"),
            ("m:@\"memory\";f<&m.Allocator>:(){a:m.heap;->&a}", "E303"),
        ] {
            assert_eq!(
                crate::compile(source).unwrap_err()[0].code,
                code,
                "{source}"
            );
        }
    }
}
