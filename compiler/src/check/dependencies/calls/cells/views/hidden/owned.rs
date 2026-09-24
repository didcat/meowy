use super::{Cells, Checker, Diagnostic, Expr, MAX_DEPTH, Result, ShapeKey, Type};

impl Checker {
    pub(in crate::check::dependencies::calls::cells) fn owned_union_cells(
        &mut self,
        arg: &Expr,
        ty: &Type,
        prefix: &[usize],
        result: &Type,
        depth: usize,
    ) -> Result<Option<Cells>> {
        let Some(paths) = self.hidden_union_paths(ty, result, arg)? else {
            return Ok(None);
        };
        let mut cells = Cells {
            complete: true,
            ..Cells::default()
        };
        for path in paths {
            let level = prefix.len() + path.key.fields.len() + path.key.variants.len();
            if level > MAX_DEPTH {
                return Err(Diagnostic::unsupported(
                    "proof owned union cell depth exhausted",
                    arg.span,
                ));
            }
            let mut fields = prefix.to_vec();
            fields.extend(&path.key.fields);
            let variants = path
                .key
                .variants
                .iter()
                .map(|(at, ty)| (at + prefix.len(), ty))
                .collect::<Vec<_>>();
            let key = ShapeKey::new(&fields, &variants, &mut self.flow, arg.span)?;
            let source = self.record_shape_source_at(arg, &key, depth + 1)?.cells;
            let Some(source) = self.hidden_path_cells(source, &path, result, arg, level)? else {
                return Ok(None);
            };
            self.merge_returned_cells(&mut cells, source, arg)?;
        }
        Ok(Some(cells))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::dependencies::{carriers::id, tests::statements};
    use crate::hir::Stmt;
    use std::collections::BTreeSet;

    #[test]
    pub(crate) fn owned_union_cell_candidates_keep_layouts_null_and_unknowns() {
        for (init, complete, extra) in [
            ("{->a:&two}", true, true),
            ("{->a:false;->b:&two}", true, true),
            ("null", true, false),
            ("{->a:unknown}", false, false),
        ] {
            let source = format!(
                "<R>:<{{r<&boolean>}}>;<A>:<{{a<&R>}}>;<B>:<{{a<boolean>;b<&R>}}>;<U>:<A><B><null>;f<&R>:(p<U>,q<&R>){{->q}};x:=false;y:=true;one<R>:{{->r:&x}};two<R>:{{->r:&y}};unknown<&R>:{{->&two}};wide<U>:{init};copy:wide;view:f(copy,&one);out:view.r"
            );
            crate::compile(&source).unwrap();
            let mut checker = Checker::new();
            statements(&mut checker, &source);
            let cells = &checker.reference_cells[&id(&checker, "view")];
            assert_eq!(cells.complete, complete);
            let mut places = BTreeSet::from([(id(&checker, "one"), vec![])]);
            let mut roots = BTreeSet::from([id(&checker, "x")]);
            if extra {
                places.insert((id(&checker, "two"), vec![]));
                roots.insert(id(&checker, "y"));
            }
            assert_eq!(cells.places, places);
            let origins = &checker.pointees[&id(&checker, "out")];
            assert_eq!(origins.complete, complete);
            assert_eq!(origins.roots, roots);
        }
    }

    #[test]
    pub(crate) fn owned_union_cell_candidates_keep_deeper_and_union_results() {
        for (result, arg, content, read) in [
            ("&U", "&one", "&two", "*view"),
            ("& &U", "&a", "&b", "**view"),
        ] {
            let source = format!(
                "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<U>:<A><B>;<C>:<{{view<{result}>}}>;<D>:<{{other<boolean>}}>;<V>:<C><D>;<W>:<{{inner<V>}}> ;make<V>:(p<{result}>){{->{{->view:p}}}};f<{result}>:(p<W>,q<{result}>){{->q}};x:=false;y:=true;one<U>:{{->r:&x}};two<U>:{{->r:&y}};a:&one;b:&two;view:f({{->inner:make({content})}},{arg});copy:{read};|copy<A>|out:copy.r"
            );
            crate::compile(&source).unwrap();
            let mut checker = Checker::new();
            statements(&mut checker, &source);
            let out = checker.locals.len() - 1;
            assert!(checker.pointees[&out].complete);
            assert_eq!(
                checker.pointees[&out].roots,
                BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
            );
            checker.mark_derived(id(&checker, "y"));
            assert!(checker.derived_local(out));
        }
    }

    #[test]
    pub(crate) fn owned_union_cell_candidates_resolve_typed_continuations() {
        let source = "<R>:<{r<&boolean>}>;<N>:<{view<&R>}>;<A>:<{nested<&N>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<&R>:(p<U>,q<&R>){->q};x:=false;y:=true;one<R>:{->r:&x};two<R>:{->r:&y};nested<N>:{->view:&two};wide<U>:{->nested:&nested};view:f(wide,&one);out:view.r";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let origins = &checker.pointees[&id(&checker, "out")];
        assert!(origins.complete);
        assert_eq!(
            origins.roots,
            BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
        );
    }

    #[test]
    pub(crate) fn owned_union_cell_queries_keep_budgets_and_no_replay() {
        let source = "<A>:<{c<& &boolean>}>;<B>:<{other<boolean>}>;<U>:<A><B>;make<U>:(p<& &boolean>){->{->c:p}};f<& &boolean>:(p<U>,q<& &boolean>){->q};x:=false;a:&x;cell:f(make(&a),&a)";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        let stmts = statements(&mut checker, source);
        let Stmt::Bind { value, .. } = stmts.last().unwrap() else {
            panic!()
        };
        let calls = checker.calls;
        assert!(checker.reference_cell(value).unwrap().complete);
        assert_eq!(checker.calls, calls);
        assert_eq!(
            checker.reference_cell_at(value, 32).err().unwrap().code,
            "B001"
        );
        assert!(!checker.flow.spend(usize::MAX));
        assert!(checker.reference_cell(value).is_err());
    }

    #[test]
    pub(crate) fn owned_union_cells_preserve_owner_loans_and_expiry() {
        let prefix = "<R>:<{r<&boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<&R>:(p<U>,q<&R>){->q};x:=false;y:=true;one<R>:{->r:&x}";
        let source = format!(
            "{prefix};two<R>:={{->r:&y}};wide<U>:{{->view:&two}};view:f(wide,&one);two={{->r:&x}};out:view.r"
        );
        assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
        let source = format!(
            "{prefix};view:{{two<R>:{{->r:&y}};wide<U>:{{->view:&two}};->f(wide,&one)}};out:view.r"
        );
        assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
    }
}
