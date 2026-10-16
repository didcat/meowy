use super::super::direct::Directs;
use super::*;
use crate::check::dependencies::edges::forward::consumers::{
    dispatch::Source as Dispatch, field_results::lookup::narrowing::Source,
    scalars::Source as Block,
};
use crate::flow::Flow;

mod expand;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Root {
    pub(crate) owner: usize,
    pub(crate) slot: Slot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Visit {
    Root(Root),
    Slot(Root),
    Unknown(Root),
    Empty(Root),
    Shared(Root),
    Cycle(Root),
    Value(Key, Input),
    Field(Key, Input, Source),
    Block(Key, Input, Block),
    Dispatch(Key, Input, Dispatch),
    Projection(Key, Input),
    Unresolved(Key, Input),
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Walk {
    pub(crate) visits: Vec<Visit>,
}

pub(super) enum Frame {
    Enter(Root),
    Next(Root, usize),
}

pub(super) struct State {
    pub(super) walk: Walk,
    pub(super) seen: BTreeMap<Root, bool>,
    pub(super) pending: Vec<Frame>,
    pub(super) limit: usize,
}

impl State {
    pub(super) fn new(items: usize) -> Self {
        Self {
            walk: Walk::default(),
            seen: BTreeMap::new(),
            pending: Vec::new(),
            limit: items.min(MAX_EDGES),
        }
    }

    pub(super) fn finish(self) -> (Walk, usize) {
        let parts = self.limit - self.walk.visits.len() - self.seen.len();
        (self.walk, parts)
    }

    pub(super) fn room(&self, span: Span) -> Result<()> {
        if self.walk.visits.len() + self.seen.len() + self.pending.len() >= self.limit {
            return Err(Diagnostic::unsupported(
                "proof candidate-walk budget exhausted",
                span,
            ));
        }
        Ok(())
    }

    pub(super) fn visit(&mut self, visit: Visit, span: Span) -> Result<()> {
        self.room(span)?;
        self.walk.visits.push(visit);
        Ok(())
    }

    pub(super) fn push(&mut self, frame: Frame, span: Span) -> Result<()> {
        self.room(span)?;
        self.pending.push(frame);
        Ok(())
    }
}

impl Graph<'_> {
    pub(super) fn forest(
        &self,
        flow: &mut Flow,
        span: Span,
        items: usize,
    ) -> Result<(Walk, usize)> {
        self.forest_sources(None, flow, span, items)
    }

    pub(in super::super) fn forest_sources(
        &self,
        sources: Option<&Directs>,
        flow: &mut Flow,
        span: Span,
        items: usize,
    ) -> Result<(Walk, usize)> {
        let roots = self.results.iter().flat_map(|(&block, (owner, result))| {
            result
                .slots
                .iter()
                .flatten()
                .enumerate()
                .map(move |(index, _)| Root {
                    owner: *owner,
                    slot: Slot { block, index },
                })
        });
        self.walk_sources(roots, sources, flow, span, items)
    }

    #[cfg(test)]
    pub(super) fn walk_roots(
        &self,
        roots: impl Iterator<Item = Root>,
        flow: &mut Flow,
        span: Span,
        items: usize,
    ) -> Result<(Walk, usize)> {
        self.walk_sources(roots, None, flow, span, items)
    }

    pub(super) fn walk_sources(
        &self,
        roots: impl Iterator<Item = Root>,
        sources: Option<&Directs>,
        flow: &mut Flow,
        span: Span,
        items: usize,
    ) -> Result<(Walk, usize)> {
        let mut state = State::new(items);
        for root in roots {
            self.walk_root(root, &mut state, sources, flow, span)?;
        }
        Ok(state.finish())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod seeded;

#[cfg(test)]
mod limits;

#[cfg(test)]
mod fields;

#[cfg(test)]
mod field_cycles;

#[cfg(test)]
mod field_limits;

#[cfg(test)]
mod blocks;

#[cfg(test)]
mod block_limits;

#[cfg(test)]
mod dispatch;

#[cfg(test)]
mod dispatch_limits;

#[cfg(test)]
mod arguments;
