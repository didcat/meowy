use crate::ast::{Expr, ExprKind};
use crate::check::{Checker, Result, Value};
use crate::hir::Type;

impl Checker {
    pub(crate) fn subtraction_operand_form(
        &mut self,
        expr: &Expr,
        depth: usize,
        count: &mut usize,
    ) -> Result<()> {
        match &expr.kind {
            ExprKind::Group(value) => {
                self.form_work(expr, depth, count)?;
                self.subtraction_operand_form(value, depth + 1, count)
            }
            ExprKind::Block(block) => {
                self.form_work(expr, depth, count)?;
                self.required_block_form(block, "type subtraction", depth, count)
            }
            _ if self.type_operand_form(expr, depth, count)? => Ok(()),
            _ => Err(Self::error(
                "E222",
                "type subtraction requires compile-time type operands",
                expr.span,
            )),
        }
    }

    pub(crate) fn subtraction_operand(&mut self, expr: &Expr) -> Result<Type> {
        if !Self::equality_block(expr) {
            return self.type_value(expr);
        }
        match self.inferred_block(expr)? {
            Value::Type(ty) => Ok(ty),
            _ => Err(Self::error(
                "E207",
                "type subtraction block must produce a type value",
                expr.span,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub(crate) fn type_subtraction_removes_normalized_alternatives_without_storage() {
        for (expr, expected) in [
            ("<int32><null>!<null>", "<int32>"),
            ("<int32>!<uint32>", "<int32>"),
            ("<int32>!<int32>", "<never>"),
            ("<never>!<int32>", "<never>"),
            ("<int32>!<never>", "<int32>"),
            ("<int32><null><boolean>!<null>!<boolean>", "<int32>"),
            ("<int32><null><boolean>!<(<null><boolean>)>", "<int32>"),
            ("<{a<int32>;b<boolean>}>!<{b<boolean>;a<int32>}>", "<never>"),
            ("<{a<int32>}>!<{a<int32>:=}>", "<{a<int32>}>"),
            ("<&int32>!<&!int32>", "<&int32>"),
            ("<int32[2]>!<int32[1+1]>", "<never>"),
            ("({local:<int32><null>;->local})!<null>", "<int32>"),
        ] {
            let source = format!(
                "kind<Type>:{expr};<T>:{{copy:kind!<never>;same:copy=={expected};|same|-><int32>}}"
            );
            let program = crate::compile(&source).unwrap();
            assert!(program.body.stmts.is_empty());
            assert!(program.locals.is_empty());
            assert!(program.functions.is_empty());
            crate::compile(&format!("{source};v<T>:7")).unwrap();
        }
        crate::compile("v<int32><null>!<null>:7").unwrap();
    }

    #[test]
    pub(crate) fn type_subtraction_keeps_kinds_queries_and_runtime_values_checked() {
        crate::compile("f<int32>:(v<uint8>){<T>:v<>!<null>;<R>:{same:<T> == <uint8>;|same|-><int32>};n<R>:7;->n}").unwrap();
        for (source, code) in [
            ("kind<Type>:1!<int32>", "E222"),
            ("kind<Type>:({->true})!<null>", "E207"),
            ("kind<Type>:<int32>!<Missing>", "E202"),
            ("core:@\"core\";kind<Type>:<int32>!<core.error>", "B001"),
            ("kind<Type>:<int32>!<1>", "E004"),
            ("v<int32><null>:null;present<(v<>!<null>)>:v", "E207"),
            ("f<int32>:(v<int32><null>){n<(v<>!<null>)>:v;->n}", "E207"),
        ] {
            let error = crate::compile(source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{source}: {error:?}");
        }
    }
}
