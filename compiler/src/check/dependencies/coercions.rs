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
    Forward,
    Convert,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Coercion {
    pub(crate) owner: usize,
    pub(crate) input: hir::PointId,
    pub(crate) kind: Kind,
    pub(crate) primary: bool,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn coercion_operation(
        &mut self,
        id: hir::PointId,
        input: hir::PointId,
        kind: Kind,
        span: Span,
    ) -> Result<()> {
        self.coercion_stages(id, input, kind, false, span)
    }

    pub(crate) fn coercion_stages(
        &mut self,
        id: hir::PointId,
        input: hir::PointId,
        kind: Kind,
        primary: bool,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof coercion-operation budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof coercion-operation identity mismatch", span);
        if !self
            .flow
            .spend(self.coercions.len().checked_ilog2().unwrap_or(0) as usize * 2 + 4)
        {
            return Err(budget());
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
                    && matches!(child.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            })
        {
            return Err(invalid());
        }
        let mut edges = vec![Edge::new(Port::Entry(id), Port::Entry(input), Route::Next)];
        let mut from = Port::Normal(input);
        if primary {
            let stage = Port::Projection { point: id, step: 0 };
            edges.push(Edge::new(from, stage, Route::Next));
            from = stage;
        }
        match kind {
            Kind::Forward => edges.push(Edge::new(from, Port::Normal(id), Route::Next)),
            Kind::Convert => edges.extend([
                Edge::new(from, Port::Operation(id), Route::Next),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
            ]),
            Kind::Stopped => (),
        }
        let op = Coercion {
            owner: self.owner,
            input,
            kind,
            primary,
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.coercions.get(&id) {
            return if *prior == op { Ok(()) } else { Err(invalid()) };
        }
        if !self.edge_room(op.edges.len()) {
            return Err(budget());
        }
        self.coercion_edges += op.edges.len();
        self.coercions.insert(id, op);
        Ok(())
    }
}

#[cfg(test)]
mod expected;

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

    pub(crate) fn prepare(source: &str) -> Checker {
        let mut checker = Checker::new();
        for stmt in crate::parser::parse(source).unwrap().stmts {
            checker.stmt(&stmt).unwrap();
        }
        checker
    }

    pub(crate) fn record() -> hir::Type {
        hir::Type::Record {
            primary: Box::new(hir::Type::Null),
            fields: Vec::new(),
        }
    }

    #[test]
    pub(crate) fn fallback_stages_distinguish_forwarding_conversion_and_inner_wrappers() {
        let int = hir::Type::Int {
            bits: 32,
            signed: true,
        };
        let union = hir::Type::union(vec![int.clone(), hir::Type::Null]);
        let mut checker = prepare("<Choice>:<int32><null>;n:7;r:{->n:1};f<int32>:(){->1}");
        for (source, target, kind) in [
            ("n", &int, Kind::Forward),
            ("n", &union, Kind::Convert),
            ("n~<Choice>", &union, Kind::Forward),
            ("f()", &union, Kind::Convert),
            ("r", &int, Kind::Forward),
            ("false&&true", &hir::Type::Bool, Kind::Forward),
        ] {
            let (id, _) = checker
                .composed_point(&expr(source), record(), Some(target))
                .unwrap();
            let op = &checker.coercions[&id];
            assert_eq!(op.kind, kind);
            assert_eq!(
                op.edges[0],
                Edge::new(Port::Entry(id), Port::Entry(op.input), Route::Next)
            );
            let tail = if kind == Kind::Forward {
                vec![Edge::new(
                    Port::Normal(op.input),
                    Port::Normal(id),
                    Route::Next,
                )]
            } else {
                vec![
                    Edge::new(Port::Normal(op.input), Port::Operation(id), Route::Next),
                    Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
                ]
            };
            assert_eq!(op.edges[1..], tail);
            assert!(!checker.region_edges.contains_key(&id));
            if source == "f()" {
                let call = checker
                    .invocations
                    .values()
                    .find(|call| call.point == op.input)
                    .unwrap();
                assert_eq!(call.edges.last().unwrap().route, Route::Returned);
            }
        }
    }

    #[test]
    pub(crate) fn fallback_stages_keep_stopped_sources_without_normal_result_bypasses() {
        let mut checker =
            prepare("a:{->1;->tag:true};d:@\"debug\";stop<never>:(){d.panic(\"stop\")}");
        let (_, value) = checker.expr_point(&expr("a==stop()"), None).unwrap();
        assert_eq!(value.ty, hir::Type::Bool);
        let (&id, op) = checker
            .coercions
            .iter()
            .find(|(_, op)| op.kind == Kind::Stopped)
            .unwrap();
        assert_eq!(
            op.edges,
            [Edge::new(
                Port::Entry(id),
                Port::Entry(op.input),
                Route::Next
            )]
        );
        assert!(
            !checker
                .invocations
                .values()
                .find(|call| call.point == op.input)
                .unwrap()
                .may_return
        );
        for target in [None, Some(&hir::Type::Null)] {
            let mut checker = prepare("d:@\"debug\"");
            let (id, _) = checker
                .composed_point(&expr("d.panic(\"stop\")"), record(), target)
                .unwrap();
            assert_eq!(checker.coercions[&id].kind, Kind::Stopped);
            assert_eq!(checker.coercions[&id].edges.len(), 1);
        }
    }

    #[test]
    pub(crate) fn fallback_stages_keep_owners_control_and_existing_failures() {
        let source = "flag:false;a:{->n:1};|flag|same:a==a;f<boolean>:(r<{n<int32>}>){->r==r}";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        checker.derived.insert(0);
        checker
            .block(&crate::parser::parse(source).unwrap(), None, None)
            .unwrap();
        assert!(checker.coercions.values().any(|op| op.owner != 0));
        assert!(
            checker
                .coercions
                .values()
                .any(|op| op.owner == 0 && op.control)
        );
        let mut checker = Checker::new();
        assert_eq!(
            checker
                .composed_point(&expr("missing"), record(), None)
                .unwrap_err()
                .code,
            "E201"
        );
        assert!(checker.coercions.is_empty());
        assert!(checker.point.is_none());
    }

    #[test]
    pub(crate) fn fallback_stages_publish_atomically_with_exact_identity_and_shared_budgets() {
        let mut checker = prepare("n:1");
        let (id, _) = checker.composed_point(&expr("n"), record(), None).unwrap();
        let op = checker.coercions[&id].clone();
        let count = checker.coercion_edges;
        checker
            .coercion_operation(id, op.input, op.kind, op.span)
            .unwrap();
        assert_eq!(checker.coercion_edges, count);
        assert!(
            checker
                .coercion_operation(id, op.input, Kind::Convert, op.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        checker.coercions.clear();
        checker.coercion_edges = 0;
        checker.points[op.input].parent = None;
        assert!(
            checker
                .coercion_operation(id, op.input, op.kind, op.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        checker.points[op.input].parent = Some(id);
        checker.dispatch_edges = super::super::edges::MAX_EDGES;
        assert!(
            checker
                .coercion_operation(id, op.input, op.kind, op.span)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert!(checker.coercions.is_empty());
        assert_eq!(checker.coercion_edges, 0);
    }
}
