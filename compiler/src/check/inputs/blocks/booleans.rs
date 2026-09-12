use crate::check::{
    Checker,
    inputs::{Input, Sources},
};
use crate::hir;

pub(crate) struct Block {
    pub(crate) target: hir::BlockId,
    pub(crate) locals: Sources,
    pub(crate) input: Input<bool>,
    pub(crate) emitted: bool,
}

impl Checker {
    pub(crate) fn boolean_block(
        &mut self,
        block: &hir::Block,
        depth: usize,
        count: &mut usize,
        locals: &Sources,
    ) -> Option<Input<bool>> {
        let mut result = Block {
            target: block.id,
            locals: locals.clone(),
            input: Input {
                work: 0,
                error: None,
                value: None,
            },
            emitted: false,
        };
        self.boolean_stmts(&block.stmts, depth, count, &mut result)?;
        (result.emitted || result.input.error.is_some()).then_some(result.input)
    }

    pub(crate) fn boolean_stmts(
        &mut self,
        stmts: &[hir::Stmt],
        depth: usize,
        count: &mut usize,
        block: &mut Block,
    ) -> Option<()> {
        for stmt in stmts {
            *count += 1;
            if *count > crate::check::type_values::MAX_WORK
                || depth >= crate::check::type_values::MAX_DEPTH
                || !self.flow.spend(1)
            {
                return None;
            }
            block.input.work = block.input.work.saturating_add(1);
            match stmt {
                hir::Stmt::Bind { id, value } if !self.proofs.mutable.contains(id) => {
                    if self.locals.get(*id) == Some(&hir::Type::Bool) {
                        let input = self.predicate_expr(value, depth, count, &block.locals)?;
                        block.input.add(&input);
                        block.locals.booleans.insert(*id, input);
                    } else {
                        let input = self.input_expr(value, depth, count, &block.locals)?;
                        block.input.add(&input);
                        block.locals.integers.insert(*id, input);
                    }
                }
                hir::Stmt::Emit {
                    target,
                    field: None,
                    value,
                    ..
                } if *target == block.target && !block.emitted => {
                    let input = self.predicate_expr(value, depth, count, &block.locals)?;
                    block.input.add(&input);
                    block.input.value = input.value;
                    block.emitted = true;
                }
                hir::Stmt::If {
                    condition,
                    then,
                    otherwise,
                } => {
                    let input = self.predicate_expr(condition, depth + 1, count, &block.locals)?;
                    block.input.add(&input);
                    if input.error.is_none() {
                        let locals = block.locals.clone();
                        let branch = if input.value? { then } else { otherwise };
                        self.boolean_stmts(branch, depth + 1, count, block)?;
                        block.locals = locals;
                    }
                }
                _ => return None,
            }
            if block.input.error.is_some() {
                block.input.value = None;
                return Some(());
            }
        }
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Span;

    #[test]
    pub(crate) fn boolean_branches_bound_active_statement_and_predicate_depth() {
        let value = hir::Expr {
            kind: hir::ExprKind::Bool(true),
            ty: hir::Type::Bool,
            span: Span { start: 0, end: 4 },
        };
        for depth in [62, 63] {
            let mut stmts = vec![hir::Stmt::Emit {
                id: 0,
                target: 0,
                field: None,
                value: value.clone(),
            }];
            for _ in 0..depth {
                stmts = vec![hir::Stmt::If {
                    condition: value.clone(),
                    then: stmts,
                    otherwise: Vec::new(),
                }];
            }
            let block = hir::Block {
                id: 0,
                ty: hir::Type::Bool,
                stmts,
            };
            let mut checker = Checker::new();
            let input = checker.boolean_block(&block, 1, &mut 0, &Sources::default());
            if depth == 62 {
                assert_eq!(input.unwrap().value, Some(true));
            } else {
                assert!(input.is_none());
            }
        }
    }
}
