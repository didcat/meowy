use super::{Checker, Origins};
use crate::{check::Result, diagnostic::Diagnostic, hir::Expr};

pub(crate) const MAX_DEPTH: usize = 64;

impl Checker {
    pub(crate) fn call_reference_origins(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        depth: usize,
    ) -> Result<Origins> {
        if !crate::borrow_contract::reference_call(&expr.ty, args.iter().map(|arg| &arg.ty)) {
            return Ok(Origins::default());
        }
        if !self.flow.spend(args.len() + 1) {
            return Err(Diagnostic::unsupported(
                "proof reference call budget exhausted",
                expr.span,
            ));
        }
        let mut origins = Origins::default();
        let mut found = false;
        for arg in args {
            if !crate::borrow_contract::returns::candidate(&expr.ty, &arg.ty) {
                continue;
            }
            let source = self.reference_origins_at(arg, depth + 1)?;
            if !self.flow.spend(source.roots.len() + 1) {
                return Err(Diagnostic::unsupported(
                    "proof reference call budget exhausted",
                    expr.span,
                ));
            }
            origins.complete = if found {
                origins.complete && source.complete
            } else {
                source.complete
            };
            found = true;
            origins.roots.extend(source.roots);
            if origins.roots.len() > super::references::MAX_ROOTS {
                return Err(Diagnostic::unsupported(
                    "proof reference origin capacity exhausted",
                    expr.span,
                ));
            }
        }
        Ok(origins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::dependencies::{carriers::id, tests::statements};
    use std::collections::BTreeSet;

    #[test]
    pub(crate) fn scalar_reference_returns_keep_compatible_argument_origins() {
        for source in [
            "f<&boolean>:(p<&boolean>){->p};x:=false;r:f(&x)",
            "f<&boolean>:(p<&boolean>){->p};x:=false;r:f(f(&x))",
            "f<&!boolean>:(p<&!boolean>){->p};x:=false;r:f(&!x)",
        ] {
            crate::compile(source).unwrap();
            let mut checker = Checker::new();
            statements(&mut checker, source);
            let x = id(&checker, "x");
            let r = id(&checker, "r");
            assert_eq!(checker.pointees[&r].roots, BTreeSet::from([x]));
            assert!(checker.pointees[&r].complete);
            checker.mark_derived(x);
            assert!(checker.derived_local(r));
        }
    }

    #[test]
    pub(crate) fn returned_origins_follow_the_contract_not_a_private_body_choice() {
        let source =
            "f<&boolean>:(a<&boolean>,b<&boolean>,n<int32>){->a};x:=false;y:=true;r:f(&x,&y,7)";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        assert_eq!(
            checker.pointees[&id(&checker, "r")].roots,
            BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
        );
        assert!(checker.pointees[&id(&checker, "r")].complete);
    }

    #[test]
    pub(crate) fn unknown_arguments_and_invalid_function_bodies_remain_checked() {
        let source = "f<&boolean>:(a<&boolean>,b<&boolean>){->a};x:=false;y:=true;r:f(&x,{->&y})";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let origins = &checker.pointees[&id(&checker, "r")];
        assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
        assert!(!origins.complete);
        let source = "f<&boolean>:(p<&boolean>){local:false;->&local};x:=false;r:f(&x)";
        assert_eq!(crate::compile(source).unwrap_err()[0].code, "E303");
    }

    #[test]
    pub(crate) fn nested_call_origin_traversal_respects_depth_and_does_not_replay_calls() {
        let mut checker = Checker::new();
        let stmts = statements(
            &mut checker,
            "f<&boolean>:(p<&boolean>){->p};x:=false;r:f(&x)",
        );
        let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
            panic!()
        };
        let calls = checker.calls;
        checker.reference_origins(value).unwrap();
        assert_eq!(checker.calls, calls);
        let error = checker
            .reference_origins_at(value, MAX_DEPTH + 1)
            .err()
            .unwrap();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("call depth"));
    }
}
