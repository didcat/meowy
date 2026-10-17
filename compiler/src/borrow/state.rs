use super::{
    BTreeMap, BTreeSet, BlockId, CallId, EmitId, Guard, Guards, LocalId, Program, ReborrowId, Span,
    State, Tags, Type,
};
use crate::borrow_value::{Projection, Source};
use crate::hir::{Place, WriteStep};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Backing {
    Result,
    Discarded,
}

#[derive(Clone)]
pub(crate) struct Alias {
    pub(crate) target: BlockId,
    pub(crate) field: String,
    pub(crate) mutable: bool,
    pub(crate) emission: EmitId,
    pub(crate) root: LocalId,
    pub(crate) span: Span,
    pub(crate) backing: Option<Backing>,
    pub(crate) borrowed: Option<Span>,
    pub(crate) exclusive: Option<Span>,
}

#[derive(Default)]
pub(crate) struct Proofs {
    pub(crate) carried: super::carried::Slots,
    pub(crate) frontiers: BTreeMap<crate::hir::RestartId, super::frontier::Frontier>,
    pub(crate) completions: BTreeMap<BlockId, Guard>,
    pub(crate) emissions: BTreeMap<EmitId, Guard>,
    pub(crate) conditions: BTreeMap<(usize, usize), Guard>,
    pub(crate) bindings: BTreeMap<LocalId, Guard>,
    pub(crate) tags: Tags,
    pub(crate) observations: BTreeMap<(usize, usize), Vec<(Type, Guard)>>,
    pub(crate) mutable: BTreeSet<LocalId>,
    pub(crate) fields: BTreeSet<LocalId>,
    pub(crate) receivers: BTreeSet<LocalId>,
    pub(crate) dispatches: BTreeSet<BlockId>,
    pub(crate) calls: BTreeMap<CallId, Guard>,
    pub(crate) aliases: BTreeMap<LocalId, Alias>,
    pub(crate) temporaries: BTreeMap<LocalId, crate::hir::StatementId>,
}

impl Proofs {
    pub(crate) fn variable(&self, id: LocalId) -> bool {
        self.mutable.contains(&id) || self.fields.contains(&id)
    }

    pub(crate) fn versioned(&self, program: &Program, id: LocalId) -> bool {
        match program.locals.get(id) {
            Some(Type::Reference(_) | Type::Exclusive(_)) => true,
            Some(Type::Foundation(crate::hir::FoundationType::Allocator)) => {
                !self.aliases.contains_key(&id)
            }
            Some(ty) if ty.fixed_borrowed_value() => {
                !self.aliases.contains_key(&id) || ty.has_reference() || self.fields.contains(&id)
            }
            _ => false,
        }
    }

    pub(crate) fn exclusive_path_type<'a>(
        &self,
        program: &'a Program,
        place: &Place,
        path: &[WriteStep],
        guards: &mut crate::flow::Flow,
        span: Span,
    ) -> Option<&'a Type> {
        if self.temporaries.contains_key(&place.root)
            || !guards.spend((place.fields.len() + path.len()).saturating_mul(4) + 1)
        {
            return None;
        }
        if let Some(alias) = self.aliases.get(&place.root)
            && (alias.exclusive.is_none() || alias.backing.is_none())
        {
            return None;
        }
        let mut ty = program.locals.get(place.root)?;
        let mut mutable = self.mutable.contains(&place.root);
        if !place.fields.is_empty() || !path.is_empty() {
            let weight = crate::borrow_contract::type_weight(ty, guards, span).ok()?;
            if !guards.spend(weight.saturating_mul(2))
                || !matches!(ty, Type::Record { .. } | Type::List { .. })
                || ty.has_reference()
                || !ty.is_copy()
            {
                return None;
            }
        }
        for index in &place.fields {
            let Type::Record { fields, .. } = ty else {
                return None;
            };
            let field = fields.get(*index)?;
            mutable = field.mutable;
            ty = &field.ty;
        }
        for step in path {
            match step {
                WriteStep::Field(index) => {
                    let Type::Record { fields, .. } = ty else {
                        return None;
                    };
                    let field = fields.get(*index)?;
                    mutable = field.mutable;
                    ty = &field.ty;
                }
                WriteStep::Index(step) => {
                    let Type::List { element, .. } = ty else {
                        return None;
                    };
                    if !matches!(step.index.ty, Type::Int { .. } | Type::Never) {
                        return None;
                    }
                    ty = element;
                }
            }
        }
        if mutable
            && path.iter().any(|step| matches!(step, WriteStep::Index(_)))
            && matches!(ty, Type::Bool | Type::Int { .. } | Type::Float { .. })
        {
            Some(ty)
        } else {
            None
        }
    }

    pub(crate) fn source(&self, place: &Place) -> Source {
        if let Some(alias) = self.aliases.get(&place.root) {
            Source::Slot {
                target: alias.target,
                root: alias.root,
                view: place.root,
                fields: place
                    .fields
                    .iter()
                    .copied()
                    .map(Projection::Field)
                    .collect(),
            }
        } else {
            Source::local(place)
        }
    }
}

#[derive(Default)]
pub(crate) struct Facts {
    pub(crate) inspections: BTreeMap<(usize, usize), Guard>,
    pub(crate) fixed_published: super::published::Fixed,
    pub(crate) changing_published: super::changing::Views,
    pub(crate) late_published: super::changing::Views,
    pub(crate) refresh_published: super::changing::Views,
    pub(crate) published_inputs: BTreeMap<BlockId, super::published::Snapshot>,
    pub(crate) published_restarts: BTreeMap<crate::hir::RestartId, super::published::Snapshot>,
    pub(crate) header_inputs: BTreeMap<BlockId, Predecessor>,
    pub(crate) restart_inputs: BTreeMap<crate::hir::RestartId, Predecessor>,
    pub(crate) headers: BTreeMap<BlockId, BTreeMap<LocalId, State>>,
    pub(crate) merging: BTreeSet<BlockId>,
    pub(crate) locals: BTreeMap<LocalId, State>,
    pub(crate) blocks: BTreeMap<BlockId, State>,
    pub(crate) calls: BTreeMap<CallId, State>,
    pub(crate) call_inputs: BTreeMap<CallId, super::calls::Inputs>,
    pub(crate) returns: BTreeMap<CallId, Vec<crate::borrow_contract::returns::Transfer>>,
    pub(crate) reborrows: BTreeMap<ReborrowId, State>,
}

pub(crate) struct Predecessor {
    pub(crate) values: BTreeMap<LocalId, State>,
    pub(crate) entered: Guard,
}

#[derive(Clone)]
pub(crate) struct Storage {
    pub(crate) block: BlockId,
    pub(crate) state: State,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Exit {
    Leave(BlockId),
    Restart(BlockId),
}

pub(crate) struct Flow {
    pub(crate) next: bool,
    pub(crate) exits: BTreeSet<Exit>,
}

impl Flow {
    pub(crate) fn new() -> Self {
        Self {
            next: true,
            exits: BTreeSet::new(),
        }
    }

    pub(crate) fn append(&mut self, next: Self) {
        self.next = next.next;
        self.exits.extend(next.exits);
    }

    pub(crate) fn merge(&mut self, other: Self) {
        self.next |= other.next;
        self.exits.extend(other.exits);
    }
}

pub(crate) struct Value {
    pub(crate) state: State,
    pub(crate) flow: Flow,
}

pub(crate) struct Checker<'a> {
    pub(crate) choices: super::activity::Choices,
    pub(crate) headers: BTreeMap<BlockId, BTreeMap<LocalId, State>>,
    pub(crate) restarts: BTreeSet<BlockId>,
    pub(crate) targets: BTreeMap<BlockId, super::exits::Target>,
    pub(crate) merging: bool,
    pub(crate) program: &'a Program,
    pub(crate) guards: &'a mut Guards,
    pub(crate) proofs: &'a Proofs,
    pub(crate) locals: BTreeMap<LocalId, Storage>,
    pub(crate) blocks: Vec<BlockId>,
    pub(crate) types: BTreeMap<BlockId, Type>,
    pub(crate) results: BTreeMap<BlockId, State>,
    pub(crate) published: super::published::Slots,
    pub(crate) writes: BTreeMap<(BlockId, Option<String>), Guard>,
    pub(crate) scopes: Vec<Vec<LocalId>>,
    pub(crate) facts: Facts,
    pub(crate) origins: usize,
    pub(crate) assumed: Guard,
    pub(crate) assumed_scopes: Vec<Guard>,
    pub(crate) inputs: BTreeSet<LocalId>,
    pub(crate) statements: BTreeMap<crate::hir::StatementId, BlockId>,
}
