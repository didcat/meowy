use super::*;
use crate::hir::Block;

pub(in super::super) type Blocks<'a> = BTreeMap<hir::BlockId, (usize, &'a Block)>;

impl Checker {
    pub(in super::super) fn program_blocks<'a>(
        &mut self,
        program: &'a Program,
        reports: &Reports,
        span: Span,
        limit: usize,
    ) -> Result<Blocks<'a>> {
        self.program_signatures(program, reports, span)?;
        let mut blocks = Blocks::new();
        let roots = std::iter::once((0, &program.body)).chain(
            program
                .functions
                .iter()
                .map(|function| (function.id + 1, &function.body)),
        );
        for (owner, body) in roots {
            self.program_block(&mut blocks, owner, body, span, limit)?;
            let mut scan = scan::Scan {
                pending: vec![(scan::Item::Statements(&body.stmts), 0)],
            };
            while let Some(expr) = scan.next_expr(&mut self.flow, span)? {
                if let ExprKind::Block(block) = &expr.kind {
                    self.program_block(&mut blocks, owner, block, span, limit)?;
                }
            }
        }
        Ok(blocks)
    }

    pub(self) fn program_block<'a>(
        &mut self,
        blocks: &mut Blocks<'a>,
        owner: usize,
        block: &'a Block,
        span: Span,
        limit: usize,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof HIR-block index budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof HIR-block identity mismatch", span);
        if blocks.len() >= limit.min(MAX_EDGES)
            || !self.flow.spend(
                blocks.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                    + 3,
            )
        {
            return Err(budget());
        }
        let weight = crate::borrow_contract::type_weight(&block.ty, &mut self.flow, span)?;
        if !self.flow.spend(weight.saturating_add(1)) {
            return Err(budget());
        }
        let body = self.bodies.get(&block.id).ok_or_else(invalid)?;
        if blocks.contains_key(&block.id) || body.owner != owner || !body.layout.matches(&block.ty)
        {
            return Err(invalid());
        }
        blocks.insert(block.id, (owner, block));
        Ok(())
    }
}

#[cfg(test)]
mod tests;
