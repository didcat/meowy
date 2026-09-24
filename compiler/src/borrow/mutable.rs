use super::{Block, Expr, ExprKind, Guards, Program, Result, Span, State, Stmt, Type};
use crate::hir::WriteStep;

pub(crate) struct Plan {
    pub(crate) merging: bool,
    pub(crate) restarts: super::BTreeSet<super::BlockId>,
    pub(crate) fixed: super::published::Fixed,
    pub(crate) changing: super::changing::Views,
    pub(crate) late: super::changing::Views,
    pub(crate) refresh: super::changing::Views,
}

#[derive(Debug, Default)]
pub(crate) struct Publications {
    pub(crate) fixed: super::published::Fixed,
    pub(crate) changing: super::changing::Views,
    pub(crate) refresh: super::changing::Views,
}

pub(crate) enum Item<'a> {
    Statement(&'a Stmt),
    Expression(&'a Expr),
    EndBlock(super::BlockId),
}

pub(crate) fn push<'a>(
    pending: &mut Vec<(Item<'a>, usize)>,
    item: Item<'a>,
    depth: usize,
    guards: &mut Guards,
) -> Result<()> {
    if pending.len() >= 65_536 || depth >= 256 || !guards.spend(1) {
        return Err(State::budget(Span::default()));
    }
    pending.push((item, depth));
    Ok(())
}

pub(crate) fn check(
    block: &Block,
    program: &Program,
    guards: &mut Guards,
    params: &[crate::hir::LocalId],
    proofs: &super::Proofs,
) -> Result<Plan> {
    let mut pending = Vec::new();
    for stmt in block.stmts.iter().rev() {
        push(&mut pending, Item::Statement(stmt), 0, guards)?;
    }
    let mut write = None;
    let mut alias_writes = super::BTreeMap::new();
    let mut parents = super::BTreeMap::from([(block.id, None)]);
    let mut owners = vec![block.id];
    if !guards.spend(params.len() + 1) {
        return Err(State::budget(Span::default()));
    }
    let mut exclusive = params
        .iter()
        .any(|id| program.locals[*id].has_exclusive())
        .then_some(Span::default());
    if block.ty.has_exclusive() {
        exclusive = Some(Span::default());
    }
    let mut restarts = super::BTreeSet::new();
    while let Some((item, depth)) = pending.pop() {
        let lookup = proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
            + alias_writes.len().checked_ilog2().unwrap_or(0) as usize
            + 1;
        if !guards.spend(lookup) {
            return Err(State::budget(Span::default()));
        }
        let mut add = |item| push(&mut pending, item, depth + 1, guards);
        match item {
            Item::EndBlock(id) => {
                if owners.pop() != Some(id) {
                    return Err(State::budget(Span::default()));
                }
            }
            Item::Statement(stmt) => match stmt {
                Stmt::Statement { stmts, .. } => {
                    for stmt in stmts.iter().rev() {
                        add(Item::Statement(stmt))?;
                    }
                }
                Stmt::Assign { id, value } => {
                    if let Some(alias) = proofs.aliases.get(id)
                        && proofs.versioned(program, *id)
                        && program.locals[*id].has_borrowed()
                        && alias.backing != Some(super::Backing::Discarded)
                    {
                        let owner = *owners.last().ok_or_else(|| State::budget(value.span))?;
                        alias_writes
                            .entry((alias.target, owner, *id))
                            .or_insert(value.span);
                    }
                    if proofs.versioned(program, *id) {
                        write.get_or_insert(value.span);
                    }
                    add(Item::Expression(value))?;
                }
                Stmt::Bind { value, .. } | Stmt::Emit { value, .. } | Stmt::Expr(value) => {
                    add(Item::Expression(value))?;
                }
                Stmt::Store { target, value, .. } => {
                    add(Item::Expression(value))?;
                    add(Item::Expression(target))?;
                }
                Stmt::SetPath {
                    id, path, value, ..
                } => {
                    if let Some(alias) = proofs.aliases.get(id)
                        && proofs.versioned(program, *id)
                        && program.locals[*id].has_borrowed()
                        && alias.backing != Some(super::Backing::Discarded)
                    {
                        let owner = *owners.last().ok_or_else(|| State::budget(value.span))?;
                        alias_writes
                            .entry((alias.target, owner, *id))
                            .or_insert(value.span);
                    }
                    if proofs.versioned(program, *id) {
                        write.get_or_insert(value.span);
                    }
                    add(Item::Expression(value))?;
                    for step in path.iter().rev() {
                        if let WriteStep::Index(step) = step {
                            add(Item::Expression(&step.index))?;
                        }
                    }
                }
                Stmt::If {
                    condition,
                    then,
                    otherwise,
                    ..
                } => {
                    for stmt in otherwise.iter().chain(then).rev() {
                        add(Item::Statement(stmt))?;
                    }
                    add(Item::Expression(condition))?;
                }
                Stmt::Restart { target, site } => {
                    if !guards
                        .spend(proofs.frontiers.len().checked_ilog2().unwrap_or(0) as usize + 1)
                    {
                        return Err(State::budget(Span::default()));
                    }
                    if proofs
                        .frontiers
                        .get(site)
                        .is_none_or(|frontier| frontier.target != *target)
                    {
                        return Err(crate::diagnostic::Diagnostic::unsupported(
                            "missing restart initialization frontier",
                            Span::default(),
                        ));
                    }
                    restarts.insert(*target);
                }
                Stmt::Leave(_) => {}
                Stmt::SlotAlias { .. } => {}
            },
            Item::Expression(expr) => {
                if expr.ty.has_exclusive() {
                    exclusive.get_or_insert(expr.span);
                    if !matches!(expr.ty, Type::Exclusive(_)) {
                        return Err(crate::diagnostic::Diagnostic::unsupported(
                            "exclusive reference carriers",
                            expr.span,
                        ));
                    }
                }
                match &expr.kind {
                    ExprKind::Block(block) => {
                        add(Item::EndBlock(block.id))?;
                        for stmt in block.stmts.iter().rev() {
                            add(Item::Statement(stmt))?;
                        }
                        if parents.len() >= 65_536
                            || !guards
                                .spend(parents.len().checked_ilog2().unwrap_or(0) as usize + 2)
                            || parents.insert(block.id, owners.last().copied()).is_some()
                        {
                            return Err(State::budget(expr.span));
                        }
                        owners.push(block.id);
                    }
                    ExprKind::Binary { left, right, .. } => {
                        if left.ty.has_exclusive() || right.ty.has_exclusive() {
                            return Err(crate::diagnostic::Diagnostic::unsupported(
                                "exclusive reference comparison",
                                expr.span,
                            ));
                        }
                        add(Item::Expression(right))?;
                        add(Item::Expression(left))?;
                    }
                    ExprKind::TemporaryBorrow { value, .. }
                    | ExprKind::Reborrow { value, .. }
                    | ExprKind::Deref(value)
                    | ExprKind::Unary { value, .. }
                    | ExprKind::Field { value, .. }
                    | ExprKind::Primary(value)
                    | ExprKind::StringSize(value)
                    | ExprKind::ListSize(value)
                    | ExprKind::Coerce { value }
                    | ExprKind::TypeTest { value, .. } => add(Item::Expression(value))?,
                    ExprKind::ExclusivePath { path, .. } => {
                        exclusive.get_or_insert(expr.span);
                        for step in path.iter().rev() {
                            if let WriteStep::Index(step) = step {
                                add(Item::Expression(&step.index))?;
                            }
                        }
                    }
                    ExprKind::ElementBorrow { value, index, .. }
                    | ExprKind::ListIndex { value, index }
                    | ExprKind::ListAdd { value, item: index } => {
                        add(Item::Expression(index))?;
                        add(Item::Expression(value))?;
                    }
                    ExprKind::List { values, .. }
                    | ExprKind::Call { args: values, .. }
                    | ExprKind::Print { parts: values, .. }
                    | ExprKind::Panic { parts: values } => {
                        for value in values.iter().rev() {
                            add(Item::Expression(value))?;
                        }
                    }
                    ExprKind::Null
                    | ExprKind::Bool(_)
                    | ExprKind::Int(_)
                    | ExprKind::Float(_)
                    | ExprKind::String(_)
                    | ExprKind::Heap
                    | ExprKind::Local(_)
                    | ExprKind::Borrow(_) => {}
                }
            }
        }
    }
    if owners != [block.id] {
        return Err(State::budget(Span::default()));
    }
    let publications = alias_restarts(&parents, &alias_writes, &restarts, guards)?;
    let mut late = super::changing::Views::new();
    for (target, ids) in &publications.changing {
        if !guards.spend(ids.len() + 1) {
            return Err(State::budget(Span::default()));
        }
        for id in ids {
            if super::frontier::late(proofs, *target, *id, guards, Span::default())? {
                if !guards.spend(late.len().checked_ilog2().unwrap_or(0) as usize + ids.len() + 2) {
                    return Err(State::budget(Span::default()));
                }
                late.entry(*target).or_default().insert(*id);
            }
        }
    }
    if let Some(span) = exclusive
        && !restarts.is_empty()
        && proofs.carried.is_empty()
    {
        return Err(crate::diagnostic::Diagnostic::unsupported(
            "exclusive references in restart bodies",
            span,
        ));
    }
    Ok(Plan {
        merging: write.is_some() || exclusive.is_some(),
        restarts,
        fixed: publications.fixed,
        changing: publications.changing,
        late,
        refresh: publications.refresh,
    })
}

pub(crate) fn alias_restarts(
    parents: &super::BTreeMap<super::BlockId, Option<super::BlockId>>,
    writes: &super::BTreeMap<(super::BlockId, super::BlockId, crate::hir::LocalId), Span>,
    restarts: &super::BTreeSet<super::BlockId>,
    guards: &mut Guards,
) -> Result<Publications> {
    let lookup = parents.len().checked_ilog2().unwrap_or(0) as usize
        + writes.len().checked_ilog2().unwrap_or(0) as usize
        + 2;
    let mut owners = super::BTreeMap::new();
    for (target, owner, _) in writes.keys() {
        if !guards.spend(lookup) || !parents.contains_key(target) {
            return Err(State::budget(Span::default()));
        }
        if !owners.contains_key(owner) {
            owners.insert(*owner, ancestors(parents, *owner, guards)?);
        }
    }
    let mut result = Publications::default();
    for id in restarts {
        let mut above = ancestors(parents, *id, guards)?;
        above.remove(id);
        let mut changed = super::BTreeSet::new();
        for ((target, owner, local), span) in writes {
            if !guards.spend(lookup + above.len() + owners[owner].len() + 1) {
                return Err(State::budget(*span));
            }
            if !above.contains(target) {
                continue;
            }
            if owners[owner].contains(id) {
                result.changing.entry(*id).or_default().insert(*local);
                changed.insert(*target);
                let mut current = *id;
                loop {
                    if !guards.spend(lookup + 2) {
                        return Err(State::budget(*span));
                    }
                    result.refresh.entry(current).or_default().insert(*local);
                    if current == *target {
                        break;
                    }
                    current = parents
                        .get(&current)
                        .copied()
                        .flatten()
                        .ok_or_else(|| State::budget(*span))?;
                }
            } else {
                result.fixed.entry(*id).or_default().insert(*target);
            }
        }
        if let Some(targets) = result.fixed.get_mut(id) {
            targets.retain(|target| !changed.contains(target));
        }
    }
    result.fixed.retain(|_, targets| !targets.is_empty());
    Ok(result)
}

pub(crate) fn ancestors(
    parents: &super::BTreeMap<super::BlockId, Option<super::BlockId>>,
    id: super::BlockId,
    guards: &mut Guards,
) -> Result<super::BTreeSet<super::BlockId>> {
    let lookup = parents.len().checked_ilog2().unwrap_or(0) as usize + 2;
    let mut result = super::BTreeSet::new();
    let mut current = id;
    loop {
        if result.len() >= 256
            || !guards.spend(lookup + result.len() + 1)
            || !result.insert(current)
        {
            return Err(State::budget(Span::default()));
        }
        let parent = parents
            .get(&current)
            .ok_or_else(|| State::budget(Span::default()))?;
        let Some(parent) = parent else {
            return Ok(result);
        };
        current = *parent;
    }
}
