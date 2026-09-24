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
pub(crate) enum Source {
    Stopped,
    Place(hir::Place),
    View,
    Temporary {
        local: hir::LocalId,
        statement: hir::StatementId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Access {
    pub(crate) position: hir::PointId,
    pub(crate) capacity: usize,
    pub(crate) length: Option<usize>,
    pub(crate) site: usize,
    pub(crate) may_return: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Element {
    pub(crate) owner: usize,
    pub(crate) parent: hir::PointId,
    pub(crate) source: Source,
    pub(crate) access: Option<Access>,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn element_operation(
        &mut self,
        id: hir::PointId,
        parent: hir::PointId,
        value: &hir::Expr,
        access: Option<Access>,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof element-borrow budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof element-borrow identity mismatch", span);
        let fields = if let hir::ExprKind::Borrow(place) = &value.kind {
            place.fields.len()
        } else {
            0
        };
        if !self.flow.spend(
            self.elements.len().checked_ilog2().unwrap_or(0) as usize * 2
                + self.proofs.temporaries.len().checked_ilog2().unwrap_or(0) as usize
                + fields
                + 6,
        ) {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if point.kind != PointKind::Expr
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
        {
            return Err(invalid());
        }
        for child in std::iter::once(parent).chain(access.map(|access| access.position)) {
            if !self.points.get(child).is_some_and(|child| {
                child.parent == Some(id)
                    && child.owner == self.owner
                    && child.block == point.block
                    && child.complete
                    && matches!(child.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            }) {
                return Err(invalid());
            }
        }
        let source = match &value.kind {
            _ if value.ty == hir::Type::Never => Source::Stopped,
            hir::ExprKind::Borrow(place) => {
                if self.locals.get(place.root).is_none() {
                    return Err(invalid());
                }
                Source::Place(place.clone())
            }
            hir::ExprKind::TemporaryBorrow { id, statement, .. } => {
                if self.locals.get(*id).is_none()
                    || self.proofs.temporaries.get(id) != Some(statement)
                {
                    return Err(invalid());
                }
                Source::Temporary {
                    local: *id,
                    statement: *statement,
                }
            }
            _ => Source::View,
        };
        let mut edges = vec![Edge::new(Port::Entry(id), Port::Entry(parent), Route::Next)];
        if let Some(access) = access {
            let hir::Type::Reference(list) = &value.ty else {
                return Err(invalid());
            };
            if !matches!(list.as_ref(), hir::Type::List { capacity, .. } if *capacity == access.capacity)
                || access.position == parent
                || access.site >= self.reborrows
                || access.length.is_some_and(|length| length > access.capacity)
            {
                return Err(invalid());
            }
            let address = Port::Address { point: id, step: 0 };
            edges.extend([
                Edge::new(Port::Normal(parent), address, Route::Next),
                Edge::new(address, Port::Entry(access.position), Route::Next),
            ]);
            if access.may_return {
                edges.extend([
                    Edge::new(
                        Port::Normal(access.position),
                        Port::Operation(id),
                        Route::Checked,
                    ),
                    Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
                ]);
            }
        } else if source != Source::Stopped {
            return Err(invalid());
        }
        let element = Element {
            owner: self.owner,
            parent,
            source,
            access,
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.elements.get(&id) {
            return if *prior == element {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(element.edges.len()) {
            return Err(budget());
        }
        self.element_edges += element.edges.len();
        self.elements.insert(id, element);
        Ok(())
    }
}

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
    pub(crate) fn element_sources_keep_places_views_and_temporary_ownership_distinct() {
        for (source, kind) in [
            ("xs:[1];p:&(xs[1])", 0),
            ("xs:[1];v:&xs;p:&(v[1])", 1),
            ("n:*(&([1][1]))", 2),
        ] {
            crate::compile(source).unwrap();
            let checker = check(source);
            let (&id, element) = checker.elements.first_key_value().unwrap();
            let access = element.access.unwrap();
            match (&element.source, kind) {
                (Source::Place(_), 0) | (Source::View, 1) => {}
                (Source::Temporary { local, statement }, 2) => {
                    assert_eq!(checker.proofs.temporaries[local], *statement)
                }
                _ => panic!(),
            }
            assert!(element.edges.contains(&Edge::new(
                Port::Normal(element.parent),
                Port::Address { point: id, step: 0 },
                Route::Next
            )));
            assert!(element.edges.contains(&Edge::new(
                Port::Address { point: id, step: 0 },
                Port::Entry(access.position),
                Route::Next
            )));
            assert!(element.edges.contains(&Edge::new(
                Port::Normal(access.position),
                Port::Operation(id),
                Route::Checked
            )));
            assert!(
                !element
                    .edges
                    .iter()
                    .any(|edge| matches!(edge.to, Port::Snapshot(_)))
            );
        }
        let source = "rows:[{->xs:[1]}];n:*(&(rows[1].xs[1]))";
        crate::compile(source).unwrap();
        assert_eq!(check(source).elements.len(), 2);
    }

    #[test]
    pub(crate) fn element_borrows_preserve_errors_and_nonreturning_operands() {
        for (source, code) in [
            ("xs:[1];p:&(xs[0])", "E101"),
            ("xs:[1];p:&(xs[false])", "E222"),
            ("p:&([1][1]);x:*p", "E303"),
            ("xs:=[1];p:&(xs[1]);xs[1]=2;x:*p", "E302"),
        ] {
            assert_eq!(crate::compile(source).unwrap_err()[0].code, code);
        }
        for source in [
            "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};p:&(stop()[missing])",
            "xs:[1];'out{p:&(xs[{'out.leave()}])}",
        ] {
            crate::compile(source).unwrap();
            let checker = check(source);
            let (&id, element) = checker.elements.first_key_value().unwrap();
            assert!(!element.edges.iter().any(|edge| edge.to == Port::Normal(id)));
        }
    }

    #[test]
    pub(crate) fn element_graph_publication_respects_shared_budgets_and_identity() {
        let mut checker = check("xs:[1];p:&(xs[1])");
        let (&id, element) = checker.elements.first_key_value().unwrap();
        let element = element.clone();
        let Source::Place(place) = &element.source else {
            panic!()
        };
        let value = hir::Expr {
            kind: hir::ExprKind::Borrow(place.clone()),
            ty: hir::Type::Reference(Box::new(checker.locals[place.root].clone())),
            span: element.span,
        };
        checker
            .element_operation(id, element.parent, &value, element.access, element.span)
            .unwrap();
        checker.elements.clear();
        checker.element_edges = 0;
        checker.points[element.parent].parent = None;
        assert!(
            checker
                .element_operation(id, element.parent, &value, element.access, element.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        checker.points[element.parent].parent = Some(id);
        checker.index_edges = super::super::edges::MAX_EDGES;
        assert!(
            checker
                .element_operation(id, element.parent, &value, element.access, element.span)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert!(checker.elements.is_empty());
        assert_eq!(checker.element_edges, 0);
    }
}
