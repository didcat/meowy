use super::*;

impl Graph<'_> {
    pub(super) fn walk_root(
        &self,
        root: Root,
        state: &mut State,
        sources: Option<&Directs>,
        flow: &mut Flow,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof candidate-walk budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof candidate-walk identity mismatch", span);
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
                    if !flow.spend(self.inputs.len().checked_ilog2().unwrap_or(0) as usize + 5) {
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
                                if !flow
                                    .spend(sources.len().checked_ilog2().unwrap_or(0) as usize + 5)
                                {
                                    return Err(budget());
                                }
                                let &(stored_owner, direct) =
                                    sources.get(&key).ok_or_else(invalid)?;
                                if stored_owner != owner || direct.point != input.point {
                                    return Err(invalid());
                                }
                                (direct.source, direct.block, direct.dispatch)
                            } else {
                                (None, None, None)
                            };
                            let (visit, slot) = match source {
                                (Some(source), None, None) => {
                                    (Visit::Field(key, input, source), Some(source.slot))
                                }
                                (None, Some(source), None) if source.slot.index == 0 => {
                                    (Visit::Block(key, input, source), Some(source.slot))
                                }
                                (None, None, Some(source)) if source.slot.index == 0 => {
                                    (Visit::Dispatch(key, input, source), Some(source.slot))
                                }
                                (None, None, None) => (Visit::Value(key, input), None),
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
        Ok(())
    }
}
