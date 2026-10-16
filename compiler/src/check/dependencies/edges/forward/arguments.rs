use super::{
    entries::{MAX_ENTRIES, MAX_REPORT_ITEMS, Reports},
    *,
};
use crate::hir;

mod forests;
mod index;
pub(super) mod sources;
mod validation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Argument {
    pub(crate) site: hir::CallId,
    pub(crate) position: usize,
    pub(crate) param: hir::LocalId,
    pub(crate) owner: usize,
    pub(crate) entry: bool,
    pub(crate) normal: bool,
}

pub(crate) type Arguments = BTreeMap<PointId, Argument>;

impl Checker {
    pub(super) fn call_arguments(
        &mut self,
        reports: &Reports,
        span: Span,
        limit: usize,
        mut parts: usize,
    ) -> Result<(Arguments, usize)> {
        let budget = || Diagnostic::unsupported("proof call-argument budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof call-argument identity mismatch", span);
        if reports.entries.len() > MAX_ENTRIES || !self.flow.spend(reports.entries.len() + 1) {
            return Err(budget());
        }
        let mut args = self.call_argument_index(reports, span, MAX_EDGES)?;
        if args.is_empty() {
            return Ok((args, parts));
        }
        let mut ports = 0usize;
        let mut count = 0;
        for (&owner, (_, walk)) in &reports.entries {
            ports = ports
                .checked_add(walk.ports.len())
                .filter(|ports| *ports <= MAX_REPORT_ITEMS)
                .ok_or_else(budget)?;
            if !self.flow.spend(walk.ports.len() + 1) {
                return Err(budget());
            }
            for &port in &walk.ports {
                let (id, normal) = match port {
                    Port::Entry(id) => (id, false),
                    Port::Normal(id) => (id, true),
                    _ => continue,
                };
                if !self
                    .flow
                    .spend(args.len().checked_ilog2().unwrap_or(0) as usize + 1)
                {
                    return Err(budget());
                }
                let Some(arg) = args.get_mut(&id) else {
                    continue;
                };
                if arg.owner != owner || self.port_owner(port, span)? != owner {
                    return Err(invalid());
                }
                if !arg.entry && !arg.normal {
                    if count >= limit.min(MAX_EDGES) {
                        return Err(budget());
                    }
                    parts = parts.checked_sub(1).ok_or_else(budget)?;
                    count += 1;
                }
                if normal {
                    arg.normal = true;
                } else {
                    arg.entry = true;
                }
            }
        }
        if !self.flow.spend(args.len() + 1) {
            return Err(budget());
        }
        args.retain(|_, arg| arg.entry || arg.normal);
        Ok((args, parts))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
