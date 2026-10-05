use super::super::direct::Directs;
use super::*;
use crate::check::dependencies::edges::forward::consumers::{
    field_results::lookup::narrowing::Source, scalars::Source as Block,
};
use crate::flow::Flow;

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
        let budget = || Diagnostic::unsupported("proof candidate-walk budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof candidate-walk identity mismatch", span);
        let mut state = State {
            walk: Walk::default(),
            seen: BTreeMap::new(),
            pending: Vec::new(),
            limit: items.min(MAX_EDGES),
        };
        for root in roots {
            if !flow.spend(1) {
                return Err(budget());
            }
            state.visit(Visit::Root(root), span)?;
            state.push(Frame::Enter(root), span)?;
            while let Some(frame) = state.pending.pop() {
                if !flow.spend(
                    self.results.len().checked_ilog2().unwrap_or(0) as usize
                        + state.seen.len().checked_ilog2().unwrap_or(0) as usize * 2
                        + 6,
                ) {
                    return Err(budget());
                }
                let root = match frame {
                    Frame::Enter(root) | Frame::Next(root, _) => root,
                };
                let (owner, result) = self.results.get(&root.slot.block).ok_or_else(invalid)?;
                if *owner != root.owner {
                    return Err(invalid());
                }
                let source = result
                    .slots
                    .as_ref()
                    .and_then(|slots| slots.get(root.slot.index))
                    .ok_or_else(invalid)?;
                match frame {
                    Frame::Enter(root) => {
                        if let Some(&active) = state.seen.get(&root) {
                            state.visit(
                                if active {
                                    Visit::Cycle(root)
                                } else {
                                    Visit::Shared(root)
                                },
                                span,
                            )?;
                            continue;
                        }
                        state.room(span)?;
                        state.seen.insert(
                            root,
                            matches!(source, Sources::Candidates(values) if !values.is_empty()),
                        );
                        state.visit(Visit::Slot(root), span)?;
                        match source {
                            Sources::Unknown => state.visit(Visit::Unknown(root), span)?,
                            Sources::Candidates(values) if values.is_empty() => {
                                state.visit(Visit::Empty(root), span)?
                            }
                            Sources::Candidates(_) => state.push(Frame::Next(root, 0), span)?,
                        }
                    }
                    Frame::Next(root, position) => {
                        let Sources::Candidates(values) = source else {
                            return Err(invalid());
                        };
                        if position == values.len() {
                            *state.seen.get_mut(&root).ok_or_else(invalid)? = false;
                            continue;
                        }
                        if !flow.spend(self.inputs.len().checked_ilog2().unwrap_or(0) as usize + 5)
                        {
                            return Err(budget());
                        }
                        let key = (root.slot.block, root.slot.index, position);
                        let &(owner, input) = self.inputs.get(&key).ok_or_else(invalid)?;
                        if owner != root.owner || values.get(position) != Some(&input.candidate) {
                            return Err(invalid());
                        }
                        state.push(Frame::Next(root, position + 1), span)?;
                        match (input.projection, input.source) {
                            (Projection::Value, None) => {
                                let source = if let Some(sources) = sources {
                                    if !flow.spend(
                                        sources.len().checked_ilog2().unwrap_or(0) as usize + 4,
                                    ) {
                                        return Err(budget());
                                    }
                                    let &(stored_owner, direct) =
                                        sources.get(&key).ok_or_else(invalid)?;
                                    if stored_owner != owner || direct.point != input.point {
                                        return Err(invalid());
                                    }
                                    (direct.source, direct.block)
                                } else {
                                    (None, None)
                                };
                                let (visit, slot) = match source {
                                    (Some(source), None) => {
                                        (Visit::Field(key, input, source), Some(source.slot))
                                    }
                                    (None, Some(source)) if source.slot.index == 0 => {
                                        (Visit::Block(key, input, source), Some(source.slot))
                                    }
                                    (None, None) => (Visit::Value(key, input), None),
                                    _ => return Err(invalid()),
                                };
                                state.visit(visit, span)?;
                                if let Some(slot) = slot {
                                    state.push(Frame::Enter(Root { owner, slot }), span)?;
                                }
                            }
                            (Projection::Value, Some(_)) => return Err(invalid()),
                            (_, Some(slot)) => {
                                state.visit(Visit::Projection(key, input), span)?;
                                state.push(Frame::Enter(Root { owner, slot }), span)?;
                            }
                            (_, None) => state.visit(Visit::Unresolved(key, input), span)?,
                        }
                    }
                }
            }
        }
        let parts = state.limit - state.walk.visits.len() - state.seen.len();
        Ok((state.walk, parts))
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
