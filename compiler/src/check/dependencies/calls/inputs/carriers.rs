use super::{Checker, Diagnostic, Expr, Origins};
use crate::check::{Result, dependencies::references::MAX_ROOTS};

impl Checker {
    pub(super) fn call_carrier_origins(&mut self, arg: &Expr, path: &[usize]) -> Result<Origins> {
        let cells = if path.is_empty() {
            self.reference_cell(arg)?
        } else {
            self.record_source_cells(arg, path)?
        };
        let mut origins = Origins {
            roots: Default::default(),
            complete: cells.complete,
        };
        for (root, path) in cells.places {
            let source = self.cell_origins(root, &path);
            if !self
                .flow
                .spend(path.len() + source.map_or(0, |source| source.roots.len()) + 1)
            {
                return Err(Diagnostic::unsupported(
                    "proof reference call cell budget exhausted",
                    arg.span,
                ));
            }
            let source = self.cell_origins(root, &path);
            origins.complete &= source.is_some_and(|source| source.complete);
            if let Some(source) = source {
                origins.roots.extend(&source.roots);
            }
            if origins.roots.len() > MAX_ROOTS {
                return Err(Diagnostic::unsupported(
                    "proof reference origin capacity exhausted",
                    arg.span,
                ));
            }
        }
        Ok(origins)
    }
}

#[cfg(test)]
mod tests {
    use crate::check::{
        Checker,
        dependencies::{carriers::id, tests::statements},
    };
    use std::collections::BTreeSet;

    #[test]
    pub(crate) fn shared_carrier_arguments_keep_inner_view_owners() {
        for tail in [
            "p:&x;r:f(&p)",
            "p:&x;c:&p;copy:c;r:f(copy)",
            "r:f(&(&x))",
            "p:&x;a:{->c:&p};r:f(a.c)",
        ] {
            let source = format!("f<&boolean>:(p<& &boolean>){{->*p}};x:=false;{tail}");
            crate::compile(&source).unwrap();
            let mut checker = Checker::new();
            statements(&mut checker, &source);
            let x = id(&checker, "x");
            let r = id(&checker, "r");
            assert_eq!(checker.pointees[&r].roots, BTreeSet::from([x]), "{tail}");
            assert!(checker.pointees[&r].complete, "{tail}");
            checker.mark_derived(x);
            assert!(checker.derived_local(r));
        }
    }

    #[test]
    pub(crate) fn record_stored_carriers_and_list_projections_keep_owners() {
        for source in [
            "<R>:<{c<& &boolean>}>;f<&boolean>:(p<R>){->*(p.c)};x:=false;p:&x;r:f({->c:&p})",
            "f<&boolean>:(p<& &boolean[2]>){->&((*p)[1])};x<boolean[2]>:=[false,true];p:&x;r:f(&p)",
        ] {
            crate::compile(source).unwrap();
            let mut checker = Checker::new();
            statements(&mut checker, source);
            let origins = &checker.pointees[&id(&checker, "r")];
            assert!(origins.complete);
            assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
        }
    }

    #[test]
    pub(crate) fn retargeted_carriers_keep_every_possible_owner() {
        let source =
            "f<&boolean>:(p<& &boolean>){->*p};x:=false;y:=true;a:&x;b:&y;c:=&a;c=&b;r:f(c)";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let origins = &checker.pointees[&id(&checker, "r")];
        assert!(origins.complete);
        assert_eq!(
            origins.roots,
            BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
        );
    }

    #[test]
    pub(crate) fn unknown_cells_or_pointees_keep_matching_candidates_incomplete() {
        for arg in ["{->&p}", "&({->&x})"] {
            let source = format!(
                "f<&boolean>:(a<& &boolean>,b<&boolean>){{->b}};x:=false;y:=true;p:&x;r:f({arg},&y)"
            );
            crate::compile(&source).unwrap();
            let mut checker = Checker::new();
            statements(&mut checker, &source);
            let origins = &checker.pointees[&id(&checker, "r")];
            assert_eq!(origins.roots, BTreeSet::from([id(&checker, "y")]));
            assert!(!origins.complete);
        }
        let source = "f<&boolean>:(p<& &boolean>){local:false;->&local};x:=false;p:&x;r:f(&p)";
        assert_eq!(crate::compile(source).unwrap_err()[0].code, "E303");
    }

    #[test]
    pub(crate) fn deeper_carrier_arguments_remain_incomplete() {
        let source = "f<&boolean>:(p<& & &boolean>){->**p};x:=false;a:&x;b:&a;r:f(&b)";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        assert!(!checker.pointees[&id(&checker, "r")].complete);
    }
}
