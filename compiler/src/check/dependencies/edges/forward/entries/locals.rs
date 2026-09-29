use super::*;

impl Checker {
    pub(super) fn report_local_count(
        &mut self,
        program: &crate::hir::Program,
        span: Span,
    ) -> Result<usize> {
        self.report_local_count_limited(program, span, MAX_EDGES)
    }

    pub(self) fn report_local_count_limited(
        &mut self,
        program: &crate::hir::Program,
        span: Span,
        limit: usize,
    ) -> Result<usize> {
        let budget =
            || Diagnostic::unsupported("proof read-storage context budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof read-storage identity mismatch", span);
        if self.local_reads.len() > limit || !self.flow.spend(self.local_reads.len() + 1) {
            return Err(budget());
        }
        for read in self.local_reads.values() {
            if !self
                .flow
                .spend(self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize + 4)
            {
                return Err(budget());
            }
            let storage = self
                .proofs
                .aliases
                .get(&read.local)
                .map_or(read.local, |alias| alias.root);
            if read.local >= program.locals.len()
                || read.storage >= program.locals.len()
                || read.storage != storage
            {
                return Err(invalid());
            }
        }
        Ok(program.locals.len())
    }
}

#[cfg(test)]
mod tests {
    use super::{super::tests::checked, *};

    #[test]
    pub(crate) fn read_storage_context_survives_hir_transfer_and_keeps_alias_roots() {
        for source in [
            "n:=1;p:&n;q:p;copy:*q;f<int32>:(x<int32>){->x}",
            "flag:=false;row:'out{|flag|{'out->n:1;copy:n};|!flag|{'out->n:2;copy:n}}",
        ] {
            crate::compile(source).unwrap();
            let (mut checker, program) = checked(source);
            assert!(checker.locals.is_empty());
            let reports = checker.entry_reports(&program, Span::default()).unwrap();
            assert_eq!(reports.locals, program.locals.len());
            for read in checker.local_reads.values() {
                assert!(read.local < reports.locals && read.storage < reports.locals);
                assert_eq!(
                    read.storage,
                    checker
                        .proofs
                        .aliases
                        .get(&read.local)
                        .map_or(read.local, |alias| alias.root)
                );
            }
        }
        let (mut checker, program) = checked("");
        assert_eq!(
            checker
                .entry_reports(&program, Span::default())
                .unwrap()
                .locals,
            0
        );
    }

    #[test]
    pub(crate) fn read_storage_context_rejects_missing_locals_and_mismatched_alias_storage() {
        for fault in 0..4 {
            let (mut checker, mut program) = checked("row:{->n:1;copy:n}");
            let (&id, read) = checker.local_reads.first_key_value().unwrap();
            let local = read.local;
            match fault {
                0 => program.locals.clear(),
                1 => checker.local_reads.get_mut(&id).unwrap().local = usize::MAX,
                2 => checker.local_reads.get_mut(&id).unwrap().storage = usize::MAX,
                3 => checker.proofs.aliases.get_mut(&local).unwrap().root = program.locals.len(),
                _ => unreachable!(),
            }
            assert_eq!(
                checker
                    .report_local_count(&program, Span::default())
                    .unwrap_err()
                    .code,
                "B001"
            );
        }
        let (mut checker, program) = checked("n:1;copy:n");
        let mut reports = checker.entry_reports(&program, Span::default()).unwrap();
        let effects = reports.effects.clone();
        reports.locals = 0;
        assert!(
            checker
                .operation_effects(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("read-effect identity")
        );
        assert_eq!(reports.effects, effects);
    }

    #[test]
    pub(crate) fn read_storage_context_bounds_capacity_and_exact_work() {
        let (mut checker, program) = checked("n:1;a:n;b:n");
        let count = checker.local_reads.len();
        assert_eq!(count, 2);
        assert!(
            checker
                .report_local_count_limited(&program, Span::default(), count - 1)
                .unwrap_err()
                .message
                .contains("budget")
        );
        let before = checker.flow.work;
        assert_eq!(
            checker
                .report_local_count_limited(&program, Span::default(), count)
                .unwrap(),
            program.locals.len()
        );
        let work = checker.flow.work - before;
        let edges = checker.edge_counts();
        for spare in [0, 1] {
            checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
            let result = checker.report_local_count(&program, Span::default());
            assert_eq!(result.is_ok(), spare == 0);
            assert_eq!(checker.edge_counts(), edges);
        }
    }
}
