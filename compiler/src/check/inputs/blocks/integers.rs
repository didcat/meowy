use super::Block;
use crate::check::{
    Checker,
    inputs::{Input, Sources},
};
use crate::hir;

impl Checker {
    pub(crate) fn input_block(
        &mut self,
        block: &hir::Block,
        depth: usize,
        count: &mut usize,
        locals: &Sources,
    ) -> Option<Input> {
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
        self.integer_stmts(&block.stmts, depth, count, &mut result)?;
        result.emitted.then_some(result.input)
    }

    pub(crate) fn integer_stmts(
        &mut self,
        stmts: &[hir::Stmt],
        depth: usize,
        count: &mut usize,
        block: &mut Block<i128>,
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
                    let input = self.input_expr(value, depth, count, &block.locals)?;
                    block.input.add(&input);
                    block.locals.integers.insert(*id, input);
                }
                hir::Stmt::Emit {
                    target,
                    field: None,
                    value,
                    ..
                } if *target == block.target && !block.emitted => {
                    let source = self.input_expr(value, depth, count, &block.locals)?;
                    block.input.add(&source);
                    block.input.value = source.value;
                    block.emitted = true;
                }
                _ => return None,
            }
        }
        Some(())
    }
}
