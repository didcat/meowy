use super::{Checker, Expr, Result, ShapeKey, Snapshot, Type};

impl Checker {
    pub(super) fn call_shape_source(
        &mut self,
        value: &Expr,
        args: &[Expr],
        key: &ShapeKey,
        depth: usize,
    ) -> Result<Snapshot> {
        let Some(ty) = self.shape_field_type(&value.ty, key, value.span)? else {
            return Ok(Snapshot::default());
        };
        let Type::Reference(target) = ty else {
            return Ok(Snapshot::default());
        };
        let mut snapshot = Snapshot::default();
        if !target.has_borrowed() {
            snapshot.origins = self.call_result_origins(ty, value.span, args, depth)?;
        }
        if self.origin_carrier(ty, value.span)? {
            snapshot.cells = self.call_result_cells(ty, value, args, depth)?;
        }
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::dependencies::{carriers::id, tests::statements};
    use std::collections::BTreeSet;

    #[test]
    pub(crate) fn union_call_results_keep_variant_specific_origins_and_copies() {
        for (variant, field, root) in [("A", "r", "x"), ("B", "r", "z")] {
            let source = format!(
                "<A>:<{{r<&boolean>}}>;<B>:<{{a<boolean>;r<&int32>}}>;<U>:<A><B>;f<U>:(p<&boolean>,q<&int32>){{->{{->r:p}}}};x:=false;z:=7;row:f(&x,&z);copy:row;|copy<{variant}>|out:copy.{field}"
            );
            crate::compile(&source).unwrap();
            let mut checker = Checker::new();
            statements(&mut checker, &source);
            let out = checker.locals.len() - 1;
            let root = id(&checker, root);
            assert!(checker.pointees[&out].complete);
            assert_eq!(checker.pointees[&out].roots, BTreeSet::from([root]));
            checker.mark_derived(root);
            assert!(checker.derived_local(out));
        }
    }

    #[test]
    pub(crate) fn union_call_results_keep_all_candidates_and_unknown_inputs() {
        for (arg, complete) in [("&y", true), ("{->&y}", false)] {
            let source = format!(
                "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<U>:<A><B>;f<U>:(p<&boolean>,q<&boolean>){{->{{->r:p}}}};x:=false;y:=true;row:f(&x,{arg});|row<A>|out:row.r"
            );
            crate::compile(&source).unwrap();
            let mut checker = Checker::new();
            statements(&mut checker, &source);
            let origins = &checker.pointees[&(checker.locals.len() - 1)];
            assert_eq!(origins.complete, complete);
            let mut roots = BTreeSet::from([id(&checker, "x")]);
            if complete {
                roots.insert(id(&checker, "y"));
            }
            assert_eq!(origins.roots, roots);
        }
    }

    #[test]
    pub(crate) fn union_call_results_keep_nested_paths_and_null_public_contracts() {
        for body in ["{->r:p}", "null"] {
            let source = format!(
                "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<U>:<A><B><null>;<W>:<{{inner<U>}}> ;f<W>:(p<&boolean>){{->inner:{body}}};g<&boolean>:(p<&boolean>){{->p}};x:=false;row:f(g(g(&x)));copy:row.inner;|copy<A>|out:copy.r"
            );
            crate::compile(&source).unwrap();
            let mut checker = Checker::new();
            statements(&mut checker, &source);
            let origins = &checker.pointees[&(checker.locals.len() - 1)];
            assert!(origins.complete);
            assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
        }
    }

    #[test]
    pub(crate) fn union_call_queries_keep_budgets_and_do_not_replay() {
        let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;f<U>:(p<&boolean>){->{->r:p}};x:=false;row:f(&x)";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        let stmts = statements(&mut checker, source);
        let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
            panic!()
        };
        let paths = checker.record_origin_paths(value, false).unwrap();
        let path = paths.iter().find(|path| matches!(path.variants[0].1, Type::Record { fields, .. } if fields[0].ty == Type::Reference(Box::new(Type::Bool)))).unwrap();
        let key =
            ShapeKey::new(&path.fields, &path.variants, &mut checker.flow, value.span).unwrap();
        let calls = checker.calls;
        assert!(
            checker
                .record_shape_source(value, &key)
                .unwrap()
                .origins
                .complete
        );
        assert_eq!(checker.calls, calls);
        assert_eq!(
            checker
                .record_shape_source_at(value, &key, 32)
                .err()
                .unwrap()
                .code,
            "B001"
        );
        assert!(!checker.flow.spend(usize::MAX));
        assert!(checker.record_shape_source(value, &key).is_err());
    }

    #[test]
    pub(crate) fn union_call_results_retain_carriers_and_preserve_lifetimes() {
        let source = "<A>:<{c<& &boolean>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<U>:(p<& &boolean>){->{->c:p}};x:=false;a:&x;row:f(&a);|row<A>|cell:row.c";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let cells = &checker.reference_cells[&(checker.locals.len() - 1)];
        assert!(cells.complete);
        assert_eq!(cells.places, BTreeSet::from([(id(&checker, "a"), vec![])]));
        let prefix =
            "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;f<U>:(p<&boolean>){->{->r:p}}";
        let source = format!("{prefix};x:=false;row:f(&x);x=true;|row<A>|out:row.r");
        assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
        let source = format!("{prefix};row:{{x:=false;->f(&x)}};|row<A>|out:row.r");
        assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
    }
}

#[cfg(test)]
mod carriers;
