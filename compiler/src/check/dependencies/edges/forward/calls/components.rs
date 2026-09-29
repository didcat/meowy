use super::*;
use crate::flow::Flow;
use std::collections::BTreeSet;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Components {
    pub(crate) groups: Vec<Group>,
    pub(crate) owners: BTreeMap<usize, usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Group {
    pub(crate) members: Vec<usize>,
    pub(crate) recursive: bool,
}

impl CallGraph {
    pub(crate) fn components(&self, flow: &mut Flow, span: Span) -> Result<Components> {
        self.components_limited(flow, span, MAX_ENTRIES, MAX_EDGES)
    }

    pub(self) fn components_limited(
        &self,
        flow: &mut Flow,
        span: Span,
        nodes: usize,
        sites: usize,
    ) -> Result<Components> {
        let budget = || Diagnostic::unsupported("proof call-component budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof call-component identity mismatch", span);
        let log = self.nodes.len().checked_ilog2().unwrap_or(0) as usize;
        if self.nodes.len() > nodes
            || self.sites.len() > sites
            || !flow.spend(self.nodes.len() * (log * 3 + 4) + 1)
        {
            return Err(budget());
        }
        let mut reverse: BTreeMap<_, Vec<_>> = self
            .nodes
            .keys()
            .map(|owner| (*owner, Vec::new()))
            .collect();
        let mut seen = BTreeSet::new();
        for (&owner, node) in &self.nodes {
            for (&target, ids) in &node.targets {
                if !flow.spend(log + 2) {
                    return Err(budget());
                }
                let parents = reverse.get_mut(&target).ok_or_else(invalid)?;
                if ids.is_empty() {
                    return Err(invalid());
                }
                for site in ids {
                    if !flow.spend(self.sites.len().checked_ilog2().unwrap_or(0) as usize * 3 + 3) {
                        return Err(budget());
                    }
                    let call = self.sites.get(site).ok_or_else(invalid)?;
                    if call.caller != owner || call.callee != target || !seen.insert(*site) {
                        return Err(invalid());
                    }
                }
                parents.push(owner);
            }
        }
        if seen.len() != self.sites.len() {
            return Err(invalid());
        }
        let order = self.finish_order(flow, span)?;
        let mut seen = BTreeSet::new();
        let mut groups = Vec::new();
        let limit = self.nodes.len() + self.sites.len();
        for &start in order.iter().rev() {
            if !flow.spend(log + 1) {
                return Err(budget());
            }
            if seen.contains(&start) {
                continue;
            }
            let mut pending = vec![start];
            let mut members = Vec::new();
            while let Some(owner) = pending.pop() {
                if !flow.spend(log * 3 + 3) {
                    return Err(budget());
                }
                if !seen.insert(owner) {
                    continue;
                }
                members.push(owner);
                for &parent in &reverse[&owner] {
                    if !flow.spend(log + 1) {
                        return Err(budget());
                    }
                    if !seen.contains(&parent) {
                        if pending.len() >= limit {
                            return Err(budget());
                        }
                        pending.push(parent);
                    }
                }
            }
            members.sort_unstable();
            let recursive = members.len() > 1 || self.nodes[&start].targets.contains_key(&start);
            groups.push(Group { members, recursive });
        }
        groups.sort_unstable_by_key(|group| group.members[0]);
        let owners = groups
            .iter()
            .enumerate()
            .flat_map(|(id, group)| group.members.iter().map(move |owner| (*owner, id)))
            .collect();
        Ok(Components { groups, owners })
    }

    pub(self) fn finish_order(&self, flow: &mut Flow, span: Span) -> Result<Vec<usize>> {
        let budget = || Diagnostic::unsupported("proof call-component budget exhausted", span);
        let log = self.nodes.len().checked_ilog2().unwrap_or(0) as usize;
        let limit = self.nodes.len() + self.sites.len();
        let mut seen = BTreeSet::new();
        let mut order = Vec::new();
        for &start in self.nodes.keys() {
            if !flow.spend(log + 1) {
                return Err(budget());
            }
            if seen.contains(&start) {
                continue;
            }
            let mut pending = vec![(start, false)];
            while let Some((owner, expanded)) = pending.pop() {
                if !flow.spend(log * 3 + 3) {
                    return Err(budget());
                }
                if expanded {
                    order.push(owner);
                    continue;
                }
                if !seen.insert(owner) {
                    continue;
                }
                pending.push((owner, true));
                for &target in self.nodes[&owner].targets.keys().rev() {
                    if !flow.spend(log + 1) {
                        return Err(budget());
                    }
                    if !seen.contains(&target) {
                        if pending.len() >= limit {
                            return Err(budget());
                        }
                        pending.push((target, false));
                    }
                }
            }
        }
        Ok(order)
    }
}

#[cfg(test)]
mod tests;
