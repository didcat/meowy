use super::PointKind;
use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir,
};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Invocation {
    pub(crate) point: hir::PointId,
    pub(crate) owner: usize,
    pub(crate) function: hir::FunctionId,
    pub(crate) site: hir::CallId,
    pub(crate) args: Vec<hir::PointId>,
    pub(crate) may_return: bool,
    pub(crate) control: bool,
    pub(crate) span: Span,
}

impl Checker {
    pub(crate) fn invocation(&mut self, call: Invocation) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof call-input budget exhausted", call.span);
        let invalid = || Diagnostic::unsupported("proof call-input identity mismatch", call.span);
        if call.args.len() > super::sequences::MAX_ITEMS
            || !self.flow.spend(
                call.args.len() * (call.args.len().checked_ilog2().unwrap_or(0) as usize + 2)
                    + self.invocations.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + self.proofs.calls.len().checked_ilog2().unwrap_or(0) as usize
                    + 4,
            )
        {
            return Err(budget());
        }
        let point = self.points.get(call.point).ok_or_else(invalid)?;
        if point.kind != PointKind::Expr
            || point.owner != self.owner
            || call.owner != self.owner
            || (!point.complete && self.point != Some(call.point))
            || self.functions.get(call.function).is_none()
            || call.site >= self.calls
            || !self.proofs.calls.contains_key(&call.site)
        {
            return Err(invalid());
        }
        let mut seen = BTreeSet::new();
        for id in &call.args {
            if !seen.insert(*id)
                || !self.points.get(*id).is_some_and(|arg| {
                    arg.parent == Some(call.point)
                        && arg.owner == call.owner
                        && arg.block == point.block
                        && arg.complete
                        && matches!(arg.kind, PointKind::Expr | PointKind::And | PointKind::Or)
                })
            {
                return Err(invalid());
            }
        }
        if let Some(prior) = self.invocations.get(&call.site) {
            return if *prior == call {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        self.invocations.insert(call.site, call);
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
    pub(crate) fn call_inputs_retain_exact_receiver_argument_and_callee_identities() {
        let source = "f<int32>:(a<int32>,b<int32>){->a+b};alias:f;x:1.(alias,2);y:alias(3,4)";
        crate::compile(source).unwrap();
        let checker = check(source);
        assert_eq!(checker.invocations.len(), 2);
        for (call, texts) in checker.invocations.values().zip([["1", "2"], ["3", "4"]]) {
            assert_eq!(call.function, 0);
            assert_eq!(call.args.len(), 2);
            assert!(call.may_return);
            for (arg, text) in call.args.iter().zip(texts) {
                let point = &checker.points[*arg];
                assert_eq!(point.parent, Some(call.point));
                assert_eq!(&source[point.span.start..point.span.end], text);
            }
        }
    }

    #[test]
    pub(crate) fn call_inputs_keep_recursion_owners_control_and_nonfunction_boundaries() {
        let source = "f<int32>:(n<int32>){->f(n)};flag:false;|flag|x:f(1);d:@\"debug\";d.print(2);p:@\"proof\";q:p.can_copy<uint8>()";
        let mut checker = Checker::new();
        let block = crate::parser::parse(source).unwrap();
        checker.derived.insert(1);
        checker.block(&block, None, None).unwrap();
        assert_eq!(checker.invocations.len(), 2);
        assert!(checker.invocations.values().any(|call| call.owner != 0));
        assert!(checker.invocations.values().any(|call| call.control));
        assert_eq!(checker.queries.len(), 1);
        for (source, code) in [
            ("f:(n<int32>){};f()", "E212"),
            ("f:(n<int32>){};f(false)", "E212"),
            ("f:1;f()", "E212"),
        ] {
            let mut checker = Checker::new();
            assert_eq!(
                checker
                    .block(&crate::parser::parse(source).unwrap(), None, None)
                    .unwrap_err()
                    .code,
                code
            );
            assert!(checker.invocations.is_empty());
        }
    }

    #[test]
    pub(crate) fn call_inputs_validate_unique_owned_roots_and_existing_sites() {
        let mut checker = check("f:(a<int32>,b<int32>){};f(1,2)");
        let call = checker.invocations.values().next().unwrap().clone();
        checker.invocation(call.clone()).unwrap();
        assert_eq!(checker.invocations.len(), 1);
        checker.invocations.clear();
        let mut cases = Vec::new();
        let mut other = call.clone();
        other.args[1] = other.args[0];
        cases.push(other);
        let mut other = call.clone();
        other.site = checker.calls;
        cases.push(other);
        let mut other = call.clone();
        other.owner += 1;
        cases.push(other);
        let mut other = call.clone();
        other.function = checker.functions.len();
        cases.push(other);
        for other in cases {
            assert!(
                checker
                    .invocation(other)
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
            assert!(checker.invocations.is_empty());
        }
        let mut other = call;
        other.args = vec![other.args[0]; super::super::sequences::MAX_ITEMS + 1];
        assert!(
            checker
                .invocation(other)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert!(checker.invocations.is_empty());
    }
}
