use super::{Checker, Result, Value};
use crate::ast::{Expr, ExprKind};
use crate::foundation::{BitOp, Item};

impl Checker {
    pub(crate) fn bits_arguments<'a>(
        &mut self,
        expr: &'a Expr,
    ) -> Result<Option<(BitOp, Vec<&'a Expr>)>> {
        let (callee, args, receiver) = match &expr.kind {
            ExprKind::Call { callee, args } => (callee, args, None),
            ExprKind::Dispatch {
                value,
                callee,
                args,
            } => (callee, args, Some(value.as_ref())),
            _ => return Ok(None),
        };
        let Some(Value::Foundation(Item::Bits(op))) = self.symbol(callee)? else {
            return Ok(None);
        };
        let args: Vec<_> = receiver.into_iter().chain(args.iter()).collect();
        let count = if op == BitOp::Not { 1 } else { 2 };
        if args.len() != count {
            return Err(Self::error(
                "E212",
                format!(
                    "`{}` expects {count} arguments, found {}",
                    op.name(),
                    args.len()
                ),
                expr.span,
            ));
        }
        Ok(Some((op, args)))
    }

    pub(crate) fn bits_expression(&mut self, expr: &Expr) -> Result<Option<Expr>> {
        let Some((op, args)) = self.bits_arguments(expr)? else {
            return Ok(None);
        };
        let kind = if op == BitOp::Not {
            ExprKind::Unary {
                op: op.operator().into(),
                value: Box::new(args[0].clone()),
            }
        } else {
            ExprKind::Binary {
                op: op.operator().into(),
                left: Box::new(args[0].clone()),
                right: Box::new(args[1].clone()),
            }
        };
        Ok(Some(Expr {
            kind,
            span: expr.span,
        }))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub(crate) fn bits_calls_resolve_aliases_dispatch_and_contextual_widths() {
        for source in [
            "b:@\"bits\";x<uint8>:b.and(255,15);y<uint8>:b.not(0);z:b.or(x,b.xor(x,y))",
            "b:@\"bits\";flip:b.not;combine:b.and;x<uint8>:3;y:1.(combine,x);z:x.(flip)",
            "b:@\"bits\";x<uint8>:1;y:b.and(255,x);z:b.and(x,255)",
            "b:@\"bits\";f<uint64>:(x<uint64>,y<uint64>){->b.or(x,b.xor(x,y))}",
            "b:@\"bits\";{b:{->and:7};value:b.and};bits:@\"debug\";bits.print(7)",
        ] {
            crate::compile(source).unwrap();
        }
    }

    #[test]
    pub(crate) fn bits_calls_reject_invalid_arity_kinds_widths_and_ranges() {
        for (body, code) in [
            ("b.and(1)", "E212"),
            ("b.not(1,2)", "E212"),
            ("b.xor()", "E212"),
            ("b.and(true,false)", "E222"),
            ("b.not(1.0)", "E222"),
            ("a<uint8>:1;c<uint16>:2;b.or(a,c)", "E213"),
            ("a<uint8>:b.and(256,0)", "E216"),
            ("a<uint8>:b.not(-1)", "E222"),
            ("b.and(1,missing)", "E201"),
            ("{b:{->and:7};b.and(1,2)}", "B001"),
        ] {
            let error = crate::compile(&format!("b:@\"bits\";{body}"))
                .unwrap_err()
                .remove(0);
            assert_eq!(error.code, code, "{body}: {error:?}");
        }
    }
}
