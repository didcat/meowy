use super::*;
use crate::check::dependencies::edges::forward::arguments::sources::Sources as Arguments;

impl Graph<'_> {
    pub(in super::super::super) fn argument_forest(
        &self,
        args: &Arguments,
        sources: &Directs,
        flow: &mut Flow,
        span: Span,
        items: usize,
    ) -> Result<(Walk, usize)> {
        let budget = || Diagnostic::unsupported("proof argument-walk budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof argument-walk identity mismatch", span);
        if args.len() > MAX_EDGES || !flow.spend(1) {
            return Err(budget());
        }
        let mut state = State::new(items);
        for (&point, &(owner, direct)) in args {
            if !flow.spend(5) {
                return Err(budget());
            }
            if point != direct.point {
                return Err(invalid());
            }
            let slot = match (direct.source, direct.block, direct.dispatch) {
                (Some(source), None, None) => Some(source.slot),
                (None, Some(source), None) if source.slot.index == 0 => Some(source.slot),
                (None, None, Some(source)) if source.slot.index == 0 => Some(source.slot),
                (None, None, None) => None,
                _ => return Err(invalid()),
            };
            state.visit(Visit::Argument(owner, direct), span)?;
            if let Some(slot) = slot {
                self.walk_root(Root { owner, slot }, &mut state, Some(sources), flow, span)?;
            }
        }
        Ok(state.finish())
    }
}
