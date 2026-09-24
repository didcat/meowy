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

pub(crate) const MAX_TARGETS: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Projection {
    Value,
    Primary,
    Field(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Target {
    pub(crate) id: hir::EmitId,
    pub(crate) block: hir::BlockId,
    pub(crate) field: Option<String>,
    pub(crate) projection: Projection,
    pub(crate) alias: Option<hir::LocalId>,
    pub(crate) storage: Option<hir::LocalId>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Emission {
    pub(crate) owner: usize,
    pub(crate) input: hir::PointId,
    pub(crate) targets: Vec<Target>,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn emission_operation(
        &mut self,
        id: hir::PointId,
        input: hir::PointId,
        from: Option<hir::LocalId>,
        stmts: &[hir::Stmt],
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof emission-operation budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof emission-operation identity mismatch", span);
        if stmts.len() > MAX_TARGETS + 2
            || !self.flow.spend(
                stmts.len()
                    * (self.frames.len()
                        + self.proofs.emissions.len().checked_ilog2().unwrap_or(0) as usize
                        + self.emission_sources.len().checked_ilog2().unwrap_or(0) as usize * 2
                        + stmts.len().checked_ilog2().unwrap_or(0) as usize
                        + 4)
                    + self.emissions.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                    + 4,
            )
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if point.kind != PointKind::Stmt
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || !self.points.get(input).is_some_and(|source| {
                source.parent == Some(id)
                    && source.owner == self.owner
                    && source.block == point.block
                    && source.complete
                    && matches!(
                        source.kind,
                        PointKind::Expr | PointKind::And | PointKind::Or
                    )
            })
        {
            return Err(invalid());
        }
        let mut targets = Vec::new();
        let mut alias = None;
        let mut seen = std::collections::BTreeSet::new();
        let mut staged = false;
        for stmt in stmts {
            match stmt {
                hir::Stmt::Emit {
                    id,
                    target,
                    field,
                    value,
                } => {
                    if (from.is_none() && !targets.is_empty())
                        || !seen.insert(*id)
                        || !self.proofs.emissions.contains_key(id)
                        || !self
                            .frames
                            .iter()
                            .any(|frame| frame.id == *target && frame.owner == self.owner)
                    {
                        return Err(invalid());
                    }
                    if !self.flow.spend(field.as_ref().map_or(0, String::len) + 1) {
                        return Err(budget());
                    }
                    if targets.len() == MAX_TARGETS {
                        return Err(budget());
                    }
                    let projection =
                        self.emission_projection(from, value, field.as_deref(), span)?;
                    targets.push(Target {
                        id: *id,
                        block: *target,
                        field: field.clone(),
                        projection,
                        alias: None,
                        storage: None,
                    });
                }
                hir::Stmt::SlotAlias { id, .. } if alias.is_none() && from.is_none() => {
                    alias = Some(*id)
                }
                hir::Stmt::Bind { id, .. } => {
                    if let Some(from) = from {
                        if *id != from || staged {
                            return Err(invalid());
                        }
                        staged = true;
                    }
                }
                _ => return Err(invalid()),
            }
        }
        if from.is_some() && !staged {
            return Err(invalid());
        }
        let target = targets.first_mut().ok_or_else(invalid)?;
        if let Some(id) = alias {
            let alias = self.proofs.aliases.get(&id).ok_or_else(invalid)?;
            if alias.emission != target.id
                || alias.target != target.block
                || target.field.as_ref() != Some(&alias.field)
            {
                return Err(invalid());
            }
            target.alias = Some(id);
            target.storage = Some(alias.root);
        }
        for (index, target) in targets.iter().enumerate() {
            if self
                .emission_sources
                .get(&target.id)
                .is_some_and(|source| *source != (id, index))
            {
                return Err(invalid());
            }
        }
        let mut edges = vec![Edge::new(Port::Entry(id), Port::Entry(input), Route::Next)];
        let mut prior = Port::Normal(input);
        for target in &targets {
            let port = Port::Emission(target.id);
            edges.push(Edge::new(prior, port, Route::Next));
            prior = port;
        }
        edges.push(Edge::new(prior, Port::Normal(id), Route::Next));
        let emission = Emission {
            owner: self.owner,
            input,
            targets,
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.emissions.get(&id) {
            return if *prior == emission {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(emission.edges.len()) {
            return Err(budget());
        }
        for (index, target) in emission.targets.iter().enumerate() {
            self.emission_sources.insert(target.id, (id, index));
        }
        self.emission_edges += emission.edges.len();
        self.emissions.insert(id, emission);
        Ok(())
    }
}

mod projections;

#[cfg(test)]
mod fanout;

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn check(source: &str) -> Checker {
        let mut checker = Checker::new();
        let block = crate::parser::parse(source).unwrap();
        checker.block(&block, None, None).unwrap();
        checker
    }

    #[test]
    pub(crate) fn emission_operations_preserve_named_primary_and_outer_slot_identities() {
        let source = "row:'out{->name:=1;{'out->2};name=3;after:4}";
        crate::compile(source).unwrap();
        let checker = check(source);
        assert_eq!(checker.emissions.len(), 2);
        for (id, emission) in &checker.emissions {
            let target = &emission.targets[0];
            assert_eq!(checker.emission_sources[&target.id], (*id, 0));
            assert_eq!(checker.points[emission.input].parent, Some(*id));
            assert_eq!(emission.edges[2].to, Port::Normal(*id));
            if target.field.is_some() {
                assert_eq!(
                    target.storage,
                    Some(checker.proofs.aliases[&target.alias.unwrap()].root)
                );
            } else {
                assert_ne!(checker.points[*id].block, Some(target.block));
            }
        }
        assert!(checker.scope_exits.is_empty());
    }

    #[test]
    pub(crate) fn emission_operations_preserve_control_and_exclude_static_or_never_outputs() {
        let mut checker = Checker::new();
        checker.derived.insert(0);
        let block = crate::parser::parse("flag:false;|flag|->1").unwrap();
        checker.block(&block, None, None).unwrap();
        assert_eq!(checker.emissions.len(), 1);
        assert!(checker.emissions.values().all(|emission| emission.control));
        let checker = check("-><T>:<uint8>;f:(){->1};'out{->{'out.leave()}}");
        assert_eq!(checker.emissions.len(), 1);
        assert!(
            checker
                .emissions
                .values()
                .all(|emission| emission.owner != 0)
        );
    }

    #[test]
    pub(crate) fn emission_operations_preserve_slot_type_and_capture_errors() {
        for (source, code) in [
            ("->x:1;->x:2", "E205"),
            ("->x<boolean>:1", "E207"),
            ("'out{f:(){'out->1}}", "E201"),
        ] {
            assert_eq!(crate::compile(source).unwrap_err()[0].code, code);
        }
    }

    #[test]
    pub(crate) fn emission_operations_validate_identity_and_publish_atomically() {
        let mut checker = Checker::new();
        let block = crate::parser::parse("->value:1").unwrap();
        checker.block_start(&block, None, None, false).unwrap();
        let (point, stmts) = checker.checked_stmt(&block.stmts[0]).unwrap();
        let input = checker.emissions[&point].input;
        let count = checker.emission_edges;
        checker
            .emission_operation(point, input, None, &stmts, block.span)
            .unwrap();
        assert_eq!(checker.emission_edges, count);
        let mut invalid = stmts.clone();
        for stmt in &mut invalid {
            if let hir::Stmt::Emit { target, .. } = stmt {
                *target = usize::MAX;
            }
        }
        assert!(
            checker
                .emission_operation(point, input, None, &invalid, block.span)
                .unwrap_err()
                .message
                .contains("identity mismatch")
        );
        assert_eq!(checker.emission_edges, count);
        checker.emissions.clear();
        checker.emission_sources.clear();
        checker.emission_edges = 0;
        checker.sequence_edges = super::super::edges::MAX_EDGES;
        assert!(
            checker
                .emission_operation(point, input, None, &stmts, block.span)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert!(checker.emissions.is_empty() && checker.emission_sources.is_empty());
        assert_eq!(checker.emission_edges, 0);
    }
}
