use super::*;
use crate::flow::Flow;
use std::collections::BTreeSet;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Condensed {
    pub(crate) nodes: Vec<Cluster>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Cluster {
    pub(crate) internal: Vec<hir::CallId>,
    pub(crate) targets: BTreeMap<usize, Vec<hir::CallId>>,
}

impl CallGraph {
    pub(crate) fn condense(
        &self,
        groups: &Components,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Condensed> {
        self.condense_limited(groups, flow, span, MAX_ENTRIES, MAX_EDGES)
    }

    pub(self) fn condense_limited(
        &self,
        groups: &Components,
        flow: &mut Flow,
        span: Span,
        nodes: usize,
        sites: usize,
    ) -> Result<Condensed> {
        let budget = || Diagnostic::unsupported("proof call-condensation budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof call-condensation identity mismatch", span);
        let log = self.nodes.len().checked_ilog2().unwrap_or(0) as usize;
        if self.nodes.len() > nodes
            || groups.groups.len() > nodes
            || self.sites.len() > sites
            || !flow.spend(self.nodes.len() * (log + 2) + groups.groups.len() + 1)
        {
            return Err(budget());
        }
        if groups.owners.len() != self.nodes.len() {
            return Err(invalid());
        }
        let mut count = 0;
        let mut previous = None;
        for (id, group) in groups.groups.iter().enumerate() {
            if group.members.is_empty() || group.members.len() > self.nodes.len() - count {
                return Err(invalid());
            }
            if !flow.spend(group.members.len() * (log * 2 + 2) + log * 2 + 4) {
                return Err(budget());
            }
            let first = group.members[0];
            if previous.is_some_and(|prior| prior >= first)
                || !group.members.windows(2).all(|pair| pair[0] < pair[1])
            {
                return Err(invalid());
            }
            for owner in &group.members {
                if !self.nodes.contains_key(owner) || groups.owners.get(owner) != Some(&id) {
                    return Err(invalid());
                }
            }
            if group.recursive
                != (group.members.len() > 1 || self.nodes[&first].targets.contains_key(&first))
            {
                return Err(invalid());
            }
            count += group.members.len();
            previous = Some(first);
        }
        if count != self.nodes.len() {
            return Err(invalid());
        }
        let mut result = Condensed {
            nodes: (0..groups.groups.len())
                .map(|_| Cluster::default())
                .collect(),
        };
        let mut seen = BTreeSet::new();
        for (&owner, node) in &self.nodes {
            let from = groups.owners[&owner];
            for (target, ids) in &node.targets {
                if ids.is_empty() || ids.len() > self.sites.len() - seen.len() {
                    return Err(invalid());
                }
                if !flow.spend(
                    ids.len() * (self.sites.len().checked_ilog2().unwrap_or(0) as usize * 3 + 3)
                        + log
                        + result.nodes[from]
                            .targets
                            .len()
                            .checked_ilog2()
                            .unwrap_or(0) as usize
                            * 2
                        + 4,
                ) {
                    return Err(budget());
                }
                let to = *groups.owners.get(target).ok_or_else(invalid)?;
                for site in ids {
                    let call = self.sites.get(site).ok_or_else(invalid)?;
                    if call.caller != owner || call.callee != *target || !seen.insert(*site) {
                        return Err(invalid());
                    }
                }
                let sites = if from == to {
                    &mut result.nodes[from].internal
                } else {
                    result.nodes[from].targets.entry(to).or_default()
                };
                sites.extend_from_slice(ids);
            }
        }
        if seen.len() != self.sites.len() {
            return Err(invalid());
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
