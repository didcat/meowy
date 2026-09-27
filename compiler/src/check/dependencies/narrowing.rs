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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Narrowing {
    pub(crate) owner: usize,
    pub(crate) input: hir::PointId,
    pub(crate) changed: bool,
    pub(crate) normal: bool,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn capture_narrowing(
        &mut self,
        id: hir::PointId,
        input: hir::PointId,
        changed: bool,
        normal: bool,
        span: Span,
    ) -> Result<()> {
        let invalid = || Diagnostic::unsupported("proof narrowing identity mismatch", span);
        if !self
            .flow
            .spend(self.narrowings.len().checked_ilog2().unwrap_or(0) as usize * 2 + 4)
        {
            return Err(Diagnostic::unsupported(
                "proof narrowing budget exhausted",
                span,
            ));
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if point.kind != PointKind::Expr
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || !self.points.get(input).is_some_and(|child| {
                child.parent == Some(id)
                    && child.owner == self.owner
                    && child.block == point.block
                    && child.complete
                    && child.kind == PointKind::Expr
            })
        {
            return Err(invalid());
        }
        let mut edges = vec![Edge::new(Port::Entry(id), Port::Entry(input), Route::Next)];
        let mut from = Port::Normal(input);
        if changed {
            edges.push(Edge::new(from, Port::Operation(id), Route::Next));
            from = Port::Operation(id);
        }
        if normal {
            edges.push(Edge::new(from, Port::Normal(id), Route::Next));
        }
        let op = Narrowing {
            owner: self.owner,
            input,
            changed,
            normal,
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.narrowings.get(&id) {
            return if *prior == op { Ok(()) } else { Err(invalid()) };
        }
        if !self.edge_room(op.edges.len()) {
            return Err(Diagnostic::unsupported(
                "proof narrowing budget exhausted",
                span,
            ));
        }
        self.narrowing_edges += op.edges.len();
        self.narrowings.insert(id, op);
        Ok(())
    }
}

#[cfg(test)]
mod stages;

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn check(source: &str) -> Checker {
        let mut checker = Checker::new();
        checker
            .block(&crate::parser::parse(source).unwrap(), None, None)
            .unwrap();
        checker
    }

    #[test]
    pub(crate) fn narrowing_roots_separate_local_and_field_sources_from_results() {
        let source =
            "f:(v<int32><null>,r<{n<int32><null>}>){|v<int32>|a<int32>:v;|r.n<int32>|b<int32>:r.n}";
        crate::compile(source).unwrap();
        let checker = check(source);
        assert_eq!(
            checker.narrowings.values().filter(|op| op.changed).count(),
            2
        );
        for (&id, op) in &checker.narrowings {
            let raw = &checker.points[op.input];
            assert_eq!(raw.parent, Some(id));
            assert_eq!(raw.owner, op.owner);
            assert_eq!(raw.block, checker.points[id].block);
            assert!(raw.complete && checker.points[id].complete);
            assert_ne!(op.owner, 0);
            if op.changed {
                let outer = checker.points[id].parent.unwrap();
                assert_eq!(checker.coercions[&outer].input, id);
                assert_eq!(
                    checker.coercions[&outer].kind,
                    super::super::CoercionKind::Forward
                );
            }
        }
        assert!(
            checker
                .narrowings
                .values()
                .any(|op| op.changed && checker.fields.contains_key(&op.input))
        );
    }

    #[test]
    pub(crate) fn narrowing_roots_keep_required_reads_and_borrow_projection_paths_separate() {
        let source = "n:2;xs<int32[n]>:[1];r:{->n:1};p:&r;x:p.n";
        crate::compile(source).unwrap();
        let checker = check(source);
        assert!(checker.narrowings.values().all(|op| !op.changed));
        assert!(checker.fields.values().any(|field| field.load));
        let mut checker = Checker::new();
        let stmt = crate::parser::parse("x:1").unwrap().stmts.remove(0);
        checker.stmt(&stmt).unwrap();
        checker.required = true;
        let crate::ast::StmtKind::Expr(expr) =
            crate::parser::parse("x").unwrap().stmts.remove(0).kind
        else {
            panic!()
        };
        checker.expr_point(&expr, None).unwrap();
        assert!(checker.narrowings.is_empty());
        assert!(checker.point.is_none());
    }
}
