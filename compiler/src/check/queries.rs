use super::{Checker, Result, Spec, Value};
use crate::ast::{Expr, ExprKind, Span};
use crate::diagnostic::Diagnostic;
use crate::foundation::Item;

pub(crate) struct Query {
    pub(crate) ty: Spec,
    pub(crate) span: Span,
    pub(crate) owner: usize,
    pub(crate) target: &'static str,
    pub(crate) revision: u32,
}

impl Query {
    pub(crate) fn unsupported(&self) -> Diagnostic {
        Diagnostic::unsupported(
            format!(
                "proof.can_copy evaluation for {} (revision {}, target {})",
                super::documentation::spec_name(&self.ty),
                self.revision,
                self.target
            ),
            self.span,
        )
    }
}

impl Checker {
    pub(crate) fn pending_query(&mut self, expr: &Expr) -> Result<Option<usize>> {
        match &expr.kind {
            ExprKind::Group(value) => self.pending_query(value),
            ExprKind::Name(_) => Ok(match self.symbol(expr)? {
                Some(Value::Pending(id)) => Some(id),
                _ => None,
            }),
            ExprKind::Call { callee, args } => {
                let ExprKind::Specialize { value, types } = &callee.kind else {
                    return Ok(None);
                };
                if !matches!(
                    self.symbol(value).ok().flatten(),
                    Some(Value::Foundation(Item::CanCopy))
                ) {
                    return Ok(None);
                }
                if types.len() != 1 || !args.is_empty() {
                    return Err(Self::error(
                        "E212",
                        "type-only proof.can_copy requires one type and no value arguments",
                        expr.span,
                    ));
                }
                let ty = self.spec(&types[0])?;
                if matches!(ty, Spec::Function { .. }) {
                    return Err(Diagnostic::unsupported(
                        "proof queries on function signatures",
                        types[0].span,
                    ));
                }
                if self.queries.len() == 4096 || !self.flow.spend(1) {
                    return Err(Diagnostic::unsupported(
                        "pending proof query capacity exhausted",
                        expr.span,
                    ));
                }
                let id = self.queries.len();
                self.queries.push(Query {
                    ty,
                    span: expr.span,
                    owner: self.owner,
                    target: crate::driver::TARGET,
                    revision: 1,
                });
                Ok(Some(id))
            }
            _ => Ok(None),
        }
    }

    pub(crate) fn pending_type(&mut self, expr: &Expr) -> Result<bool> {
        let mut expr = expr;
        while let ExprKind::Group(value) = &expr.kind {
            expr = value;
        }
        let ExprKind::TypeQuery(value) = &expr.kind else {
            return Ok(false);
        };
        let mut value = value.as_ref();
        while let ExprKind::Group(inner) = &value.kind {
            value = inner;
        }
        Ok(matches!(&value.kind, ExprKind::Name(_))
            && matches!(self.symbol(value)?, Some(Value::Pending(_))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(crate) fn pending_proof_copies_keep_fixed_types_and_original_sites() {
        let source = r#"p:@"proof";query:p.can_copy;r:query<uint32>();copy<p.Result>:r;<R>:copy<>;again<R>:copy;second:query<never>()"#;
        let block = crate::parser::parse(source).unwrap();
        let mut checker = Checker::new();
        let body = checker.block(&block, None, None).unwrap();
        assert!(body.stmts.is_empty());
        assert!(checker.locals.is_empty());
        assert_eq!(checker.queries.len(), 2);
        let query = &checker.queries[0];
        assert_eq!(&source[query.span.start..query.span.end], "query<uint32>()");
        assert_eq!(query.target, crate::driver::TARGET);
        assert_eq!(query.revision, 1);
        assert!(matches!(
            query.ty,
            Spec::Data(crate::hir::Type::Int {
                bits: 32,
                signed: false
            })
        ));
        assert!(matches!(
            checker.queries[1].ty,
            Spec::Data(crate::hir::Type::Never)
        ));
        let error = crate::compile(source).unwrap_err().remove(0);
        assert_eq!(error.code, "B001");
        assert_eq!(error.span, query.span);
        assert!(error.message.contains("proof.can_copy evaluation"));
    }

    #[test]
    pub(crate) fn pending_proof_gate_follows_ordinary_type_and_ownership_errors() {
        for (tail, code) in [
            ("bad<int32>:false", "E207"),
            ("v:missing", "E201"),
            ("x:=7;r:&x;x=8;v:*r", "E302"),
            ("f<&int32>:(){x:7;->&x}", "E303"),
        ] {
            let source = format!(r#"p:@"proof";result:p.can_copy<uint32>();{tail}"#);
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{source}: {error:?}");
            assert!(error.span.start > source.find("();").unwrap());
        }
        for source in [
            r#"p:@"proof";f:(){r:p.can_copy<uint32>();copy:r}"#,
            r#"p:@"proof";|false|{r:p.can_copy<uint32>()}"#,
        ] {
            let error = crate::compile(source).unwrap_err().remove(0);
            assert_eq!(error.code, "B001");
            assert!(error.message.contains("proof.can_copy evaluation"));
        }
    }

    #[test]
    pub(crate) fn pending_proof_values_reject_escapes_and_preserve_query_validation() {
        for tail in [
            "copy:=r",
            "copy<int32>:r",
            "f:(){copy:r}",
            "d:@\"debug\";d.print(r)",
            "->value:r",
            "|r|{}",
        ] {
            let source = format!(r#"p:@"proof";r:p.can_copy<uint32>();{tail}"#);
            assert_eq!(
                crate::compile(&source).unwrap_err()[0].code,
                "E223",
                "{source}"
            );
        }
        for (call, code) in [
            ("p.can_copy<uint32,int32>()", "E212"),
            ("p.can_copy<uint32>(missing)", "E212"),
            ("p.can_copy<Missing>()", "E202"),
            ("p.can_copy<uint8[1/0]>()", "E107"),
        ] {
            let source = format!(r#"p:@"proof";r:{call}"#);
            assert_eq!(
                crate::compile(&source).unwrap_err()[0].code,
                code,
                "{source}"
            );
        }
        let source = r#"p:@"proof";r:p.can_copy<uint32>();copy<p.Always>:r"#;
        assert_eq!(crate::compile(source).unwrap_err()[0].code, "E207");
        assert!(crate::compile(r#"p:@"debug";p.print(7)"#).is_ok());
    }
    #[test]
    pub(crate) fn pending_proof_capacity_counts_calls_and_preserves_earlier_origins() {
        let prefix = r#"p:@"proof";r:p.can_copy<uint32>();"#;
        let copies = (0..4100).map(|id| format!("r{id}:r;")).collect::<String>();
        let mut checker = Checker::new();
        checker
            .block(
                &crate::parser::parse(&format!("{prefix}{copies}")).unwrap(),
                None,
                None,
            )
            .unwrap();
        assert_eq!(checker.queries.len(), 1);
        for count in [4095, 4096, 4097] {
            let source = format!("p:@\"proof\";{}", "p.can_copy<uint32>();".repeat(count));
            let block = crate::parser::parse(&source).unwrap();
            let mut checker = Checker::new();
            let result = checker.block(&block, None, None);
            assert_eq!(result.is_ok(), count <= 4096);
            assert_eq!(checker.queries.len(), count.min(4096));
            if let Err(error) = result {
                assert_eq!(error.code, "B001");
                assert!(error.message.contains("pending proof query capacity"));
                assert_eq!(error.span.start, source.rfind("p.can_copy").unwrap());
            }
        }
    }
}
