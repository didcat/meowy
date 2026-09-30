use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Observed {
    pub(crate) place: crate::hir::Place,
    pub(crate) storage: crate::hir::LocalId,
    pub(crate) mode: crate::hir::ReferenceMode,
    pub(crate) control: bool,
    pub(crate) addresses: Vec<bool>,
    pub(crate) acquired: bool,
    pub(crate) result: bool,
}

impl Checker {
    pub(super) fn record_place_borrow_effect(
        &mut self,
        owner: usize,
        port: Port,
        effects: &mut Effects,
        limit: usize,
        parts: &mut usize,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof place-borrow-effect budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof place-borrow-effect identity mismatch", span);
        let id = match port {
            Port::Address { point, .. } | Port::Operation(point) | Port::Normal(point) => point,
            _ => return Err(invalid()),
        };
        if !self.flow.spend(
            self.place_borrows.len().checked_ilog2().unwrap_or(0) as usize
                + effects.len().checked_ilog2().unwrap_or(0) as usize * 2
                + 8,
        ) {
            return Err(budget());
        }
        let op = self.place_borrows.get(&id).ok_or_else(invalid)?;
        let len = op.place.fields.len();
        if len > crate::list::MAX_WRITE_PATH || !self.flow.spend(len * 3 + 1) {
            return Err(budget());
        }
        if op.owner != owner || matches!(port, Port::Address { step, .. } if step > len) {
            return Err(invalid());
        }
        let fresh = if let Some((prior_owner, effect)) = effects.get(&id) {
            let Effect::Borrow(prior) = effect else {
                return Err(invalid());
            };
            if *prior_owner != owner
                || prior.place != op.place
                || prior.storage != op.storage
                || prior.mode != op.mode
                || prior.control != op.control
                || prior.addresses.len() != len + 1
            {
                return Err(invalid());
            }
            false
        } else {
            if effects.len() >= limit || len * 2 + 1 > *parts {
                return Err(budget());
            }
            true
        };
        let (_, Effect::Borrow(observed)) = effects.entry(id).or_insert_with(|| {
            (
                owner,
                Effect::Borrow(Observed {
                    place: op.place.clone(),
                    storage: op.storage,
                    mode: op.mode,
                    control: op.control,
                    addresses: vec![false; len + 1],
                    acquired: false,
                    result: false,
                }),
            )
        }) else {
            unreachable!()
        };
        match port {
            Port::Address { step, .. } => observed.addresses[step] = true,
            Port::Operation(_) => observed.acquired = true,
            Port::Normal(_) => observed.result = true,
            _ => unreachable!(),
        }
        if fresh {
            *parts -= len * 2 + 1;
        }
        Ok(())
    }

    pub(super) fn validate_place_borrow(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<bool> {
        let budget = || Diagnostic::unsupported("proof place-borrow-effect budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof place-borrow-effect identity mismatch", span);
        let id = match port {
            Port::Address { point, .. } | Port::Operation(point) | Port::Normal(point) => point,
            _ => return Ok(false),
        };
        if !self
            .flow
            .spend(self.place_borrows.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some(op) = self.place_borrows.get(&id) else {
            return Ok(false);
        };
        let len = op.place.fields.len();
        if len > crate::list::MAX_WRITE_PATH
            || !self.flow.spend(
                len * 3
                    + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                    + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                    + reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize
                    + 20,
            )
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        let storage = self
            .proofs
            .aliases
            .get(&op.place.root)
            .map_or(op.place.root, |alias| alias.root);
        if op.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != op.span
            || op.place.root >= reports.locals
            || op.storage >= reports.locals
            || op.storage != storage
            || op.counts.len() != len
            || op.edges.len() != len + 3
            || reports.index.operations.get(&id) != Some(&owner)
            || !point
                .block
                .and_then(|id| self.bodies.get(&id))
                .is_some_and(|body| body.owner == owner)
            || point.parent.is_some_and(|parent| {
                parent == id
                    || !self.points.get(parent).is_some_and(|parent| {
                        parent.complete && parent.owner == owner && parent.block == point.block
                    })
            })
            || matches!(port, Port::Address { step, .. } if step > len)
        {
            return Err(invalid());
        }
        let address = |step| Port::Address { point: id, step };
        if op.edges[0] != Edge::new(Port::Entry(id), address(0), Route::Next)
            || op.edges[len + 1] != Edge::new(address(len), Port::Operation(id), Route::Next)
            || op.edges[len + 2] != Edge::new(Port::Operation(id), Port::Normal(id), Route::Next)
        {
            return Err(invalid());
        }
        for (step, (&field, &count)) in op.place.fields.iter().zip(&op.counts).enumerate() {
            if field >= count
                || op.edges[step + 1] != Edge::new(address(step), address(step + 1), Route::Next)
            {
                return Err(invalid());
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod validation;

#[cfg(test)]
mod limits;

#[cfg(test)]
mod boundaries;

#[cfg(test)]
mod tests {
    use super::{super::tests::checked, *};
    use crate::hir::ReferenceMode;

    #[test]
    pub(crate) fn place_borrow_effects_retain_places_modes_and_reference_cells() {
        for (source, mode) in [
            (
                "n:1;p:&n;cell:&p;r:{->inner:{->x:2}};q:&(r.inner.x)",
                ReferenceMode::Shared,
            ),
            (
                "n<uint8>:=7;p:&!n;r:{->inner:{->x:=2}};q:&!(r.inner.x)",
                ReferenceMode::Exclusive,
            ),
        ] {
            crate::compile(source).unwrap();
            let (checker, reports) = checked(source, false);
            assert!(checker.locals.is_empty());
            let mut cells = Vec::new();
            for (&id, op) in &checker.place_borrows {
                assert_eq!(
                    reports.effects[&id],
                    (
                        op.owner,
                        Effect::Borrow(Observed {
                            place: op.place.clone(),
                            storage: op.storage,
                            mode,
                            control: false,
                            addresses: vec![true; op.place.fields.len() + 1],
                            acquired: true,
                            result: true,
                        })
                    )
                );
                assert!(!checker.local_reads.contains_key(&id));
                assert!(!checker.derefs.contains_key(&id));
                cells.push(op.storage);
            }
            let count = cells.len();
            cells.sort_unstable();
            cells.dedup();
            assert_eq!(cells.len(), count);
        }
    }

    #[test]
    pub(crate) fn place_borrow_effects_keep_addresses_acquisition_and_results_independent() {
        let (mut checker, mut reports) = checked("r:{->inner:{->x:2}};p:&(r.inner.x)", false);
        let id = *checker.place_borrows.first_key_value().unwrap().0;
        for port in (0..=2)
            .map(|step| Port::Address { point: id, step })
            .chain([Port::Operation(id), Port::Normal(id)])
        {
            reports.entries.get_mut(&0).unwrap().1.ports = vec![port, port];
            let effects = checker
                .operation_effects_limited(&reports, Span::default(), 1, 5, 0)
                .unwrap();
            assert_eq!(effects.len(), 1);
            let (_, Effect::Borrow(op)) = &effects[&id] else {
                panic!()
            };
            for (step, &seen) in op.addresses.iter().enumerate() {
                assert_eq!(seen, port == Port::Address { point: id, step });
            }
            assert_eq!(op.acquired, port == Port::Operation(id));
            assert_eq!(op.result, port == Port::Normal(id));
        }
    }

    #[test]
    pub(crate) fn place_borrow_effects_keep_alias_storage_owners_and_control() {
        for mode in ["&", "&!"] {
            let source = format!(
                "flag:false;n:=1;|flag|p:{mode}n;f:(b<boolean>)'out{{|b|{{'out->n:=1;p:{mode}n}};|!b|{{'out->n:=2;p:{mode}n}}}}"
            );
            crate::compile(&source).unwrap();
            let (checker, reports) = checked(&source, true);
            let ops = checker
                .place_borrows
                .values()
                .filter(|op| op.owner != 0)
                .collect::<Vec<_>>();
            assert_eq!(ops.len(), 2);
            assert_ne!(ops[0].place.root, ops[1].place.root);
            assert_eq!(ops[0].storage, ops[1].storage);
            assert!(checker.place_borrows.values().any(|op| op.control));
            for (&id, op) in &checker.place_borrows {
                let (owner, Effect::Borrow(observed)) = &reports.effects[&id] else {
                    panic!()
                };
                assert_eq!(*owner, op.owner);
                assert_eq!(observed.control, op.control);
                assert_eq!(observed.place, op.place);
                assert_eq!(observed.storage, op.storage);
                assert!(observed.acquired && observed.result);
            }
        }
    }
}
