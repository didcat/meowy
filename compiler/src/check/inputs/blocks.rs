mod booleans;
mod integers;

use super::{Input, Sources};
use crate::hir;

pub(crate) struct Block<T> {
    pub(crate) target: hir::BlockId,
    pub(crate) locals: Sources,
    pub(crate) input: Input<T>,
    pub(crate) emitted: bool,
}
