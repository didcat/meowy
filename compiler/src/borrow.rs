pub(crate) mod activity;
pub(crate) mod aliases;
pub(crate) mod branches;
pub(crate) mod calls;
pub(crate) mod carried;
pub(crate) mod changing;
pub(crate) mod control;
pub(crate) mod emissions;
pub(crate) mod exits;
pub(crate) mod frontier;
pub(crate) mod header;
pub(crate) mod mutable;
pub(crate) mod origins;
pub(crate) mod pointee;
pub(crate) mod published;
pub(crate) mod replay;
pub(crate) mod restart;
pub(crate) mod state;
pub(crate) mod tags;
pub(crate) mod value;
use std::collections::{BTreeMap, BTreeSet};

use crate::ast::Span;
pub(crate) use crate::borrow_value::{Origin, Path, Projection, Source, State, Step};
use crate::diagnostic::Diagnostic;
use crate::flow::{FALSE, Flow as Guards, Guard, TRUE};
use crate::hir::{
    Block, BlockId, CallId, EmitId, Expr, ExprKind, LocalId, Program, ReborrowId, Stmt, Type,
};

pub(crate) type Result<T> = std::result::Result<T, Diagnostic>;
pub(crate) type Tags = BTreeMap<((LocalId, Vec<String>), Type), Vec<(Type, Guard)>>;
pub(crate) const MAX_ORIGINS: usize = 4_096;
pub(crate) const MAX_FACT_ORIGINS: usize = 262_144;

pub(crate) use emissions::slot;
pub(crate) use state::{Alias, Backing, Checker, Exit, Facts, Flow, Proofs, Storage, Value};

pub(crate) fn check(
    program: &Program,
    guards: &mut Guards,
    proofs: &Proofs,
) -> std::result::Result<Facts, Vec<Diagnostic>> {
    let result = replay::check(program, guards, proofs);
    if guards.exceeded() {
        Err(vec![Diagnostic::unsupported(
            "control-flow proof budget exhausted",
            Span { start: 0, end: 0 },
        )])
    } else {
        result.map_err(|error| vec![error])
    }
}

#[cfg(test)]
mod tests;
