mod carriers;
mod records;

use super::{Checker, Diagnostic, Expr, Origins};
use crate::check::dependencies::records::{MAX_DEPTH, MAX_FIELDS};
use crate::{check::Result, hir::Type};

pub(super) enum Input {
    Unsupported,
    Absent,
    Known(Origins),
}

impl Checker {
    pub(super) fn call_input_origins(
        &mut self,
        arg: &Expr,
        result: &Type,
        depth: usize,
    ) -> Result<Input> {
        let mut pending = vec![(&arg.ty, Vec::new())];
        let mut origins = Origins::default();
        let mut found = false;
        let mut visits = 0;
        while let Some((ty, path)) = pending.pop() {
            visits += 1;
            if visits > MAX_FIELDS || path.len() > MAX_DEPTH || !self.flow.spend(path.len() + 1) {
                return Err(Diagnostic::unsupported(
                    "proof reference call input budget exhausted",
                    arg.span,
                ));
            }
            if !ty.has_borrowed() {
                continue;
            }
            let source = match Self::origin_record(ty).unwrap_or(ty) {
                Type::Record { primary, fields } if !primary.has_borrowed() => {
                    if pending.len() + fields.len() > MAX_FIELDS {
                        return Err(Diagnostic::unsupported(
                            "proof reference call input capacity exhausted",
                            arg.span,
                        ));
                    }
                    for (index, field) in fields.iter().enumerate() {
                        let mut path = path.clone();
                        path.push(index);
                        pending.push((&field.ty, path));
                    }
                    continue;
                }
                Type::Union(_) => {
                    match self.call_union_input_origins(arg, ty, &path, result, depth)? {
                        Input::Unsupported => return Ok(Input::Unsupported),
                        Input::Absent => continue,
                        Input::Known(source) => source,
                    }
                }
                Type::Reference(_) => {
                    let Some((ty, layers)) = self.call_shared_view(ty, arg)? else {
                        return Ok(Input::Unsupported);
                    };
                    if ty.pointee().is_some_and(Type::has_borrowed) {
                        match self.call_record_view_origins(arg, &path, ty, result, layers)? {
                            Input::Unsupported => return Ok(Input::Unsupported),
                            Input::Absent => continue,
                            Input::Known(source) => source,
                        }
                    } else {
                        if crate::borrow_contract::projections(
                            ty,
                            result,
                            &mut self.flow,
                            arg.span,
                        )?
                        .is_empty()
                        {
                            continue;
                        }
                        if layers > 0 {
                            self.call_carrier_origins(arg, &path, layers)?
                        } else if path.is_empty() {
                            self.reference_origins_at(arg, depth + 1)?
                        } else {
                            self.record_source_origins_at(arg, &path, depth + 1)?
                        }
                    }
                }
                _ => return Ok(Input::Unsupported),
            };
            if !self.flow.spend(source.roots.len() + 1) {
                return Err(Diagnostic::unsupported(
                    "proof reference call input budget exhausted",
                    arg.span,
                ));
            }
            origins.complete = if found {
                origins.complete && source.complete
            } else {
                source.complete
            };
            found = true;
            origins.roots.extend(source.roots);
            if origins.roots.len() > crate::check::dependencies::references::MAX_ROOTS {
                return Err(Diagnostic::unsupported(
                    "proof reference origin capacity exhausted",
                    arg.span,
                ));
            }
        }
        Ok(if found {
            Input::Known(origins)
        } else {
            Input::Absent
        })
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
    pub(crate) fn record_arguments_retain_named_and_nested_reference_origins() {
        for source in [
            "<R>:<{r<&boolean>}>;f<&boolean>:(p<R>){->p.r};x:=false;r:f({->r:&x})",
            "<R>:<{r<&boolean>}>;f<&boolean>:(p<R>){->p.r};x:=false;a<R>:{->r:&x};r:f(a)",
            "<R>:<{r<&boolean>}>;<N>:<{inner<R>}>;f<&boolean>:(p<N>){->p.inner.r};x:=false;a<N>:{->inner:{->r:&x}};r:f(a)",
            "<R>:<{r<&boolean[2]>}>;f<&boolean>:(p<R>){->&(p.r[1])};x<boolean[2]>:=[false,true];r:f({->r:&x})",
        ] {
            crate::compile(source).unwrap();
            let mut checker = Checker::new();
            statements(&mut checker, source);
            let x = id(&checker, "x");
            let r = id(&checker, "r");
            assert_eq!(checker.pointees[&r].roots, BTreeSet::from([x]), "{source}");
            assert!(checker.pointees[&r].complete, "{source}");
            checker.mark_derived(x);
            assert!(checker.derived_local(r));
        }
    }

    #[test]
    pub(crate) fn record_candidates_retain_all_matching_fields_and_arguments() {
        let source = "<R>:<{a<&boolean>;b<&boolean>;n<&int32>}>;f<&boolean>:(p<R>,q<&boolean>){->p.a};x:=false;y:=true;z:=false;n:=7;r:f({->a:&x;->b:&y;->n:&n},&z)";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let origins = &checker.pointees[&id(&checker, "r")];
        assert!(origins.complete);
        assert_eq!(
            origins.roots,
            BTreeSet::from([id(&checker, "x"), id(&checker, "y"), id(&checker, "z")])
        );
    }

    #[test]
    pub(crate) fn unknown_record_candidates_preserve_known_owners_and_incompleteness() {
        let source = "<R>:<{a<&boolean>;b<&boolean>}>;f<&boolean>:(p<R>){->p.a};x:=false;y:=true;r:f({->a:&x;->b:{->&y}})";
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let origins = &checker.pointees[&id(&checker, "r")];
        assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
        assert!(!origins.complete);
    }

    #[test]
    pub(crate) fn record_call_inputs_charge_the_analysis_budget() {
        let mut checker = Checker::new();
        let stmts = statements(&mut checker, "x:=false;a:{->r:&x}");
        let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
            panic!()
        };
        assert!(!checker.flow.spend(usize::MAX));
        let error = checker
            .call_input_origins(
                value,
                &crate::hir::Type::Reference(Box::new(crate::hir::Type::Bool)),
                0,
            )
            .err()
            .unwrap();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("call input budget"));
    }
}

#[cfg(test)]
mod nullable;

mod unions;
