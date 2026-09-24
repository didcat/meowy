use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir,
};

pub(crate) const MAX_POINTS: usize = 262_144;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Expr,
    Read,
    Query,
    Match,
    And,
    Or,
    Condition,
    Then,
    Else,
}

impl Kind {
    pub(crate) fn expression(expr: &crate::ast::Expr) -> Self {
        match &expr.kind {
            crate::ast::ExprKind::Binary { op, .. } if op == "&&" => Self::And,
            crate::ast::ExprKind::Binary { op, .. } if op == "||" => Self::Or,
            _ => Self::Expr,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Point {
    pub(crate) kind: Kind,
    pub(crate) owner: usize,
    pub(crate) block: Option<hir::BlockId>,
    pub(crate) site: Option<hir::StatementId>,
    pub(crate) parent: Option<usize>,
    pub(crate) span: Span,
    pub(crate) complete: bool,
}

impl Checker {
    pub(crate) fn with_point<T>(
        &mut self,
        kind: Kind,
        span: Span,
        check: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        self.with_point_id(kind, span, check)
            .map(|(_, value)| value)
    }

    pub(crate) fn with_point_id<T>(
        &mut self,
        kind: Kind,
        span: Span,
        check: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<(hir::PointId, T)> {
        if self.points.len() >= MAX_POINTS || !self.flow.spend(self.frames.len() + 1) {
            return Err(Diagnostic::unsupported(
                "proof expression point budget exhausted",
                span,
            ));
        }
        let site = self.checked_site(span)?;
        let parent = self.point.filter(|id| self.points[*id].owner == self.owner);
        let block = self
            .frames
            .iter()
            .rev()
            .find(|frame| frame.owner == self.owner)
            .map(|frame| frame.id);
        let id = self.points.len();
        self.points.push(Point {
            kind,
            owner: self.owner,
            block,
            site,
            parent,
            span,
            complete: false,
        });
        let prior = self.point.replace(id);
        let result = check(self);
        self.point = prior;
        self.points[id].complete = result.is_ok();
        result.map(|value| (id, value))
    }
}

#[cfg(test)]
mod uses;

#[cfg(test)]
mod branches;

#[cfg(test)]
mod anchors;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(crate) fn point_results_return_the_enclosing_id_after_nested_checking() {
        let mut checker = Checker::new();
        let span = Span::default();
        let (outer, (inner, value)) = checker
            .with_point_id(Kind::Expr, span, |checker| {
                checker.with_point_id(Kind::Expr, span, |_| Ok(7))
            })
            .unwrap();
        assert_eq!(value, 7);
        assert_ne!(outer, inner);
        assert_eq!(checker.points[inner].parent, Some(outer));
        assert!(checker.points[outer].parent.is_none());
        assert!(checker.points[outer].complete);
        assert!(checker.points[inner].complete);
        assert!(checker.point.is_none());
    }

    #[test]
    pub(crate) fn point_results_restore_the_parent_after_a_failed_child() {
        let mut checker = Checker::new();
        let span = Span::default();
        let (outer, inner) = checker
            .with_point_id(Kind::Expr, span, |checker| {
                let parent = checker.point;
                let error = checker
                    .with_point_id(Kind::Expr, span, |_| {
                        Err::<(), _>(Checker::error("E207", "invalid child", span))
                    })
                    .unwrap_err();
                assert_eq!(error.code, "E207");
                assert_eq!(checker.point, parent);
                checker
                    .with_point_id(Kind::Expr, span, |_| Ok(()))
                    .map(|(id, _)| id)
            })
            .unwrap();
        assert_eq!(checker.points[inner].parent, Some(outer));
        assert_eq!(
            checker
                .points
                .iter()
                .filter(|point| !point.complete)
                .count(),
            1
        );
        assert!(checker.points[outer].complete);
        assert!(checker.point.is_none());
    }

    #[test]
    pub(crate) fn expression_points_distinguish_repeated_checks_with_identical_spans() {
        let mut checker = Checker::new();
        let stmt = crate::parser::parse("1+2").unwrap().stmts.remove(0);
        checker.stmt(&stmt).unwrap();
        checker.stmt(&stmt).unwrap();
        assert_eq!(checker.points.len(), 6);
        let points = &checker.points;
        assert_eq!(points[0].span, points[3].span);
        assert_eq!(points[0].site, Some(0));
        assert_eq!(points[3].site, Some(1));
        assert_eq!(points[1].parent, Some(0));
        assert_eq!(points[2].parent, Some(0));
        assert_eq!(points[4].parent, Some(3));
        assert_eq!(points[5].parent, Some(3));
        assert!(points.iter().all(|point| point.complete));
        assert!(checker.point.is_none());
    }

    #[test]
    pub(crate) fn expression_points_isolate_nested_function_owners() {
        let source = "x:{f:(){a:1+2};->3+4}";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        let block = crate::parser::parse(source).unwrap();
        checker.block(&block, None, None).unwrap();
        let function = checker.functions[0].as_ref().unwrap();
        for point in &checker.points {
            if let Some(parent) = point.parent {
                assert_eq!(point.owner, checker.points[parent].owner);
            }
        }
        let local = checker
            .points
            .iter()
            .find(|point| point.block == Some(function.body.id))
            .unwrap();
        assert_ne!(local.owner, checker.points[0].owner);
        assert!(local.parent.is_none());
        assert!(checker.point.is_none());
    }

    #[test]
    pub(crate) fn expression_points_restore_parents_after_coercion_errors() {
        let mut checker = Checker::new();
        let stmt = crate::parser::parse("x<boolean>:1+2")
            .unwrap()
            .stmts
            .remove(0);
        assert_eq!(checker.stmt(&stmt).unwrap_err().code, "E207");
        assert!(checker.point.is_none());
        assert!(!checker.points[0].complete);
        assert!(checker.points.iter().skip(1).all(|point| point.complete));
        let start = checker.points.len();
        let stmt = crate::parser::parse("3").unwrap().stmts.remove(0);
        checker.stmt(&stmt).unwrap();
        assert!(checker.points[start].parent.is_none());
        assert!(checker.points[start].complete);
    }

    #[test]
    pub(crate) fn expression_points_bound_capture_without_changing_logical_work() {
        let mut checker = Checker::new();
        let span = Span { start: 0, end: 1 };
        checker.points.resize_with(MAX_POINTS, || Point {
            kind: Kind::Expr,
            owner: 0,
            block: None,
            site: None,
            parent: None,
            span,
            complete: true,
        });
        let mut called = false;
        let error = checker
            .with_point(Kind::Expr, span, |_| {
                called = true;
                Ok(())
            })
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(!called);
        assert!(checker.point.is_none());
        assert!(checker.type_work.is_none());
        assert_eq!(checker.points.len(), MAX_POINTS);
    }
}
