use super::{entries::Reports, *};
use crate::hir;

mod index;

pub(crate) type Index = BTreeMap<PointId, (usize, hir::BlockId)>;
