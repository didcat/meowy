mod booleans;
mod integers;

use super::{Checker, Input, Sources};
use crate::hir;

pub(crate) struct Block<T> {
    pub(crate) target: hir::BlockId,
    pub(crate) locals: Sources,
    pub(crate) input: Input<T>,
    pub(crate) emitted: bool,
}

impl Checker {
    pub(crate) fn scalar_binding<T>(
        &mut self,
        id: hir::LocalId,
        value: &hir::Expr,
        depth: usize,
        count: &mut usize,
        block: &mut Block<T>,
    ) -> Option<()> {
        if self.proofs.mutable.contains(&id) {
            return None;
        }
        if self.locals.get(id) == Some(&hir::Type::Bool) {
            let input = self.predicate_expr(value, depth, count, &block.locals)?;
            block.input.add(&input);
            block.locals.booleans.insert(id, input);
        } else if matches!(
            self.locals.get(id),
            Some(hir::Type::Int { .. } | hir::Type::Never)
        ) {
            let input = self.input_expr(value, depth, count, &block.locals)?;
            block.input.add(&input);
            block.locals.integers.insert(id, input);
        } else {
            return None;
        }
        Some(())
    }
}
