use super::Checker;
use crate::{ast::Span, check::Result, diagnostic::Diagnostic, hir};
use std::collections::btree_map::Entry;

pub(crate) const MAX_RESTART_INPUTS: usize = 65_536;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RestartInput {
    pub(crate) target: usize,
    pub(crate) owner: usize,
    pub(crate) span: Span,
    pub(crate) control: bool,
}

impl Checker {
    pub(crate) fn track_restart_input(
        &mut self,
        site: hir::RestartId,
        target: usize,
        span: Span,
    ) -> Result<()> {
        if site >= MAX_RESTART_INPUTS
            || !self
                .flow
                .spend(self.restart_inputs.len().checked_ilog2().unwrap_or(0) as usize + 4)
        {
            return Err(Diagnostic::unsupported(
                "proof restart dependency budget exhausted",
                span,
            ));
        }
        let input = RestartInput {
            target,
            owner: self.owner,
            span,
            control: self.control,
        };
        match self.restart_inputs.entry(site) {
            Entry::Vacant(entry) => {
                entry.insert(input);
            }
            Entry::Occupied(entry) if *entry.get() == input => {}
            Entry::Occupied(_) => {
                return Err(Diagnostic::unsupported(
                    "proof restart site identity mismatch",
                    span,
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::dependencies::{carriers::id, tests::statements};

    #[test]
    pub(crate) fn restart_evidence_distinguishes_ordinary_and_derived_control() {
        let source = "flag:false;'loop{|flag|'loop.restart()};plain:3";
        crate::compile(source).unwrap();
        for derived in [false, true] {
            let mut checker = Checker::new();
            if derived {
                checker.derived.insert(0);
            }
            let block = crate::parser::parse(source).unwrap();
            checker.block(&block, None, None).unwrap();
            assert_eq!(checker.restart_inputs.len(), 1);
            let input = &checker.restart_inputs[&0];
            assert_eq!(input.control, derived);
            assert_eq!(input.target, checker.proofs.frontiers[&0].target);
            assert_eq!(input.owner, 0);
            assert_eq!(&source[input.span.start..input.span.end], "'loop.restart()");
            assert!(!checker.control);
            assert!(!checker.continuation_control());
        }
    }

    #[test]
    pub(crate) fn restart_evidence_keeps_alias_targets_and_distinct_sites() {
        let source =
            "flag:false;'outer{again:'outer.restart;'inner{|flag|again();|!flag|'inner.restart()}}";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        checker.derived.insert(0);
        let block = crate::parser::parse(source).unwrap();
        checker.block(&block, None, None).unwrap();
        assert_eq!(checker.restart_inputs.len(), 2);
        let outer = &checker.restart_inputs[&0];
        let inner = &checker.restart_inputs[&1];
        assert!(outer.control && inner.control);
        assert_ne!(outer.target, inner.target);
        assert_eq!(outer.target, checker.proofs.frontiers[&0].target);
        assert_eq!(inner.target, checker.proofs.frontiers[&1].target);
        assert_eq!(&source[outer.span.start..outer.span.end], "again()");
        assert_eq!(
            &source[inner.span.start..inner.span.end],
            "'inner.restart()"
        );
    }

    #[test]
    pub(crate) fn restart_evidence_keeps_function_owners_and_leave_continuations() {
        let source = "f<int32>:(flag<boolean>){'loop{|flag|'loop.restart()};->1};flag:false;'out{|flag|'out.leave();'loop{'loop.restart()}}";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(
            &mut checker,
            "f<int32>:(flag<boolean>){'loop{|flag|'loop.restart()};->1};flag:false",
        );
        checker.mark_derived(id(&checker, "flag"));
        statements(
            &mut checker,
            "'out{|flag|'out.leave();'loop{'loop.restart()}}",
        );
        assert_eq!(checker.restart_inputs.len(), 2);
        assert_ne!(
            checker.restart_inputs[&0].owner,
            checker.restart_inputs[&1].owner
        );
        assert!(!checker.restart_inputs[&0].control);
        assert!(checker.restart_inputs[&1].control);
        assert_eq!(checker.restart_inputs[&1].owner, 0);
    }

    #[test]
    pub(crate) fn invalid_restart_operations_do_not_record_dependency_edges() {
        for (source, code) in [
            ("'loop{'loop.restart(1)}", "E212"),
            ("'outer{f<int32>:(){'outer.restart();->1}}", "E201"),
        ] {
            let mut checker = Checker::new();
            let block = crate::parser::parse(source).unwrap();
            assert_eq!(checker.block(&block, None, None).unwrap_err().code, code);
            assert!(checker.restart_inputs.is_empty());
            assert_eq!(checker.restarts, 0);
        }
    }

    #[test]
    pub(crate) fn restart_evidence_bounds_sites_work_and_identity() {
        let mut checker = Checker::new();
        let span = Span::new(1, 2);
        checker.track_restart_input(0, 7, span).unwrap();
        checker.track_restart_input(0, 7, span).unwrap();
        assert_eq!(checker.restart_inputs.len(), 1);
        assert!(checker.track_restart_input(0, 8, span).is_err());
        assert_eq!(checker.restart_inputs[&0].target, 7);
        assert_eq!(
            checker
                .track_restart_input(MAX_RESTART_INPUTS, 7, span)
                .unwrap_err()
                .code,
            "B001"
        );
        assert!(!checker.flow.spend(usize::MAX));
        assert!(checker.track_restart_input(1, 7, span).is_err());
        assert_eq!(checker.restart_inputs.len(), 1);
    }
}
