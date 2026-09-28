use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Family {
    Branch,
    Region,
    Sequence,
    ScopeExit,
    Endpoint,
    Restart,
    Storage,
    LocalRead,
    ScalarLeaf,
    HeapLeaf,
    Path,
    Store,
    Invocation,
    Index,
    Method,
    Element,
    Exclusive,
    Output,
    Unary,
    Deref,
    Field,
    Typed,
    Coercion,
    Narrowing,
    Binary,
    Dispatch,
    Reborrow,
    Projection,
    PlaceBorrow,
    TemporaryBorrow,
    Emission,
}

pub(crate) const FAMILIES: usize = Family::Emission as usize + 1;

mod collect;

impl Checker {
    pub(crate) fn edge_counts(&self) -> [(Family, usize); FAMILIES] {
        [
            (Family::Branch, self.branch_edges.len().saturating_mul(6)),
            (Family::Region, self.region_edges.len().saturating_mul(2)),
            (Family::Sequence, self.sequence_edges),
            (Family::ScopeExit, self.scope_exits.len()),
            (Family::Endpoint, self.endpoint_edges),
            (Family::Restart, self.restart_edges.len()),
            (Family::Storage, self.operation_edges),
            (Family::LocalRead, self.local_read_edges),
            (Family::ScalarLeaf, self.scalar_leaf_edges),
            (Family::HeapLeaf, self.heap_leaf_edges),
            (Family::Path, self.path_edges),
            (Family::Store, self.store_edges),
            (Family::Invocation, self.invocation_edges),
            (Family::Index, self.index_edges),
            (Family::Method, self.method_edges),
            (Family::Element, self.element_edges),
            (Family::Exclusive, self.exclusive_edges),
            (Family::Output, self.output_edges),
            (Family::Unary, self.unary_edges),
            (Family::Deref, self.deref_edges),
            (Family::Field, self.field_edges),
            (Family::Typed, self.typed_edges),
            (Family::Coercion, self.coercion_edges),
            (Family::Narrowing, self.narrowing_edges),
            (Family::Binary, self.binary_edges),
            (Family::Dispatch, self.dispatch_edges),
            (Family::Reborrow, self.reborrow_edges),
            (Family::Projection, self.projection_edges),
            (Family::PlaceBorrow, self.place_borrow_edges),
            (Family::TemporaryBorrow, self.temporary_borrow_edges),
            (Family::Emission, self.emission_edges),
        ]
    }
}

#[cfg(test)]
mod counts;

#[cfg(test)]
pub(crate) mod coverage;
