use super::{
    PathStep, PointKind,
    edges::{Edge, Port, Route},
};
use crate::{
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Operation {
    pub(crate) owner: usize,
    pub(crate) place: hir::Place,
    pub(crate) storage: hir::LocalId,
    pub(crate) steps: Vec<PathStep>,
    pub(crate) control: bool,
    pub(crate) span: crate::ast::Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn exclusive_operation(
        &mut self,
        id: hir::PointId,
        value: &hir::Expr,
        steps: Vec<PathStep>,
    ) -> Result<()> {
        let budget =
            || Diagnostic::unsupported("proof exclusive-borrow budget exhausted", value.span);
        let invalid =
            || Diagnostic::unsupported("proof exclusive-borrow identity mismatch", value.span);
        let hir::ExprKind::ExclusivePath { place, path } = &value.kind else {
            return Err(invalid());
        };
        if steps.len() + place.fields.len() > crate::list::MAX_WRITE_PATH
            || !self.flow.spend(
                self.exclusives.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                    + steps.len() * (steps.len().checked_ilog2().unwrap_or(0) as usize + 4)
                    + place.fields.len()
                    + 4,
            )
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        let storage = self
            .proofs
            .aliases
            .get(&place.root)
            .map_or(place.root, |alias| alias.root);
        if point.kind != PointKind::Expr
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || self.locals.get(storage).is_none()
            || steps.len() != path.len()
            || !matches!(steps.first(), Some(PathStep::Index { .. }))
        {
            return Err(invalid());
        }
        let mut ty = self.locals.get(place.root).ok_or_else(invalid)?;
        for field in &place.fields {
            let hir::Type::Record { fields, .. } = ty else {
                return Err(invalid());
            };
            ty = &fields.get(*field).ok_or_else(invalid)?.ty;
        }
        let mut seen = std::collections::BTreeSet::new();
        let address = |step| Port::Address { point: id, step };
        let mut edges = vec![Edge::new(Port::Entry(id), address(0), Route::Next)];
        let mut stopped = false;
        for (index, (step, source)) in steps.iter().zip(path).enumerate() {
            match (step, source, ty) {
                (
                    PathStep::Field(field),
                    hir::WriteStep::Field(other),
                    hir::Type::Record { fields, .. },
                ) if field == other => {
                    ty = &fields.get(*field).ok_or_else(invalid)?.ty;
                    edges.push(Edge::new(address(index), address(index + 1), Route::Next));
                }
                (
                    PathStep::Index {
                        point: child,
                        capacity,
                        span,
                    },
                    hir::WriteStep::Index(source),
                    hir::Type::List {
                        element,
                        capacity: size,
                    },
                ) => {
                    if capacity != size
                        || *span != source.span
                        || !seen.insert(*child)
                        || !self.points.get(*child).is_some_and(|child| {
                            child.parent == Some(id)
                                && child.owner == self.owner
                                && child.block == point.block
                                && child.complete
                                && matches!(
                                    child.kind,
                                    PointKind::Expr | PointKind::And | PointKind::Or
                                )
                        })
                    {
                        return Err(invalid());
                    }
                    let reserve = Port::Reserve {
                        point: id,
                        step: index,
                    };
                    edges.extend([
                        Edge::new(address(index), reserve, Route::Next),
                        Edge::new(reserve, Port::Entry(*child), Route::Next),
                    ]);
                    if source.index.ty == hir::Type::Never {
                        stopped = true;
                    } else {
                        edges.push(Edge::new(
                            Port::Normal(*child),
                            address(index + 1),
                            Route::Checked,
                        ));
                    }
                    ty = element;
                }
                _ => return Err(invalid()),
            }
        }
        if stopped {
            if value.ty != hir::Type::Never {
                return Err(invalid());
            }
        } else {
            if value.ty.pointee() != Some(ty) || !matches!(value.ty, hir::Type::Exclusive(_)) {
                return Err(invalid());
            }
            edges.extend([
                Edge::new(address(steps.len()), Port::Operation(id), Route::Next),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
            ]);
        }
        let operation = Operation {
            owner: self.owner,
            place: place.clone(),
            storage,
            steps,
            control: self.control,
            span: value.span,
            edges,
        };
        if let Some(prior) = self.exclusives.get(&id) {
            return if *prior == operation {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(operation.edges.len()) {
            return Err(budget());
        }
        self.exclusive_edges += operation.edges.len();
        self.exclusives.insert(id, operation);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
