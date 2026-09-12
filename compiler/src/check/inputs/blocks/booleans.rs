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
