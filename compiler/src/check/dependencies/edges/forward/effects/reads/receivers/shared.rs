use super::*;
use crate::check::dependencies::bodies::Completion;

impl Checker {
    pub(in crate::check::dependencies::edges::forward) fn shared_receiver_input(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        ty: ScalarKind,
        span: Span,
    ) -> Result<Option<PointId>> {
        let Some(source) = self.receiver_read(reports, id, owner, span)? else {
            return Ok(None);
        };
        let Some(actual) = source.shared_primary else {
            return Ok(None);
        };
        if !matches!(source.shape, Shape::Record { .. })
            || actual != ty
            || !(Completion {
                normal: true,
                result: Shape::SharedScalar(actual),
            })
            .valid()
        {
            return Err(Diagnostic::unsupported(
                "proof shared-receiver identity mismatch",
                span,
            ));
        }
        if !self.flow.spend(
            self.proofs.mutable.len().checked_ilog2().unwrap_or(0) as usize
                + self.proofs.fields.len().checked_ilog2().unwrap_or(0) as usize
                + 2,
        ) {
            return Err(Diagnostic::unsupported(
                "proof shared-receiver budget exhausted",
                span,
            ));
        }
        if self.proofs.variable(source.local) {
            return Ok(None);
        }
        self.receiver_input(reports, id, owner, source, span)
            .map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::dependencies::edges::forward::effects::tests::checked;

    #[test]
    pub(crate) fn shared_receiver_hops_require_initialization_without_generic_record_forwarding() {
        let (mut checker, mut reports) =
            checked("n:1;p:&n;v:{->p;->tag:true}.{copy:$;->$==p}", false);
        let (&local, &(_, dispatch, kind)) = reports.receivers.first_key_value().unwrap();
        let kind = kind.unwrap();
        let input = checker.dispatch_ops[&dispatch].input;
        let reads: Vec<_> = checker
            .local_reads
            .iter()
            .filter(|(_, read)| read.local == local)
            .map(|(&id, _)| id)
            .collect();
        assert_eq!(reads.len(), 2);
        assert!(!reports.eligible.contains(&local));
        for initialized in [false, true] {
            for result in [false, true] {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                    panic!()
                };
                op.initialized = initialized;
                op.result = result;
                for &id in &reads {
                    assert_eq!(
                        checker
                            .shared_receiver_input(&reports, id, 0, kind, Span::default())
                            .unwrap(),
                        initialized.then_some(input)
                    );
                    assert_eq!(
                        checker
                            .receiver_value_input(&reports, id, 0, Span::default(), true)
                            .unwrap(),
                        None
                    );
                }
            }
        }
        for fault in 0..3 {
            reports.receivers.get_mut(&local).unwrap().2 = if fault == 0 {
                None
            } else {
                Some(ScalarKind::Bool)
            };
            if fault == 2 {
                checker
                    .dispatch_ops
                    .get_mut(&dispatch)
                    .unwrap()
                    .shared_primary = None;
            }
            assert!(
                checker
                    .shared_receiver_input(&reports, reads[0], 0, kind, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
        }
        reports.receivers.remove(&local);
        assert_eq!(
            checker
                .shared_receiver_input(&reports, reads[0], 0, kind, Span::default())
                .unwrap(),
            None
        );
    }
}
