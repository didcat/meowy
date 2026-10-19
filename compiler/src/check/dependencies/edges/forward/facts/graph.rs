use super::*;
use ports::Site;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Incoming {
    pub(super) edge: usize,
    pub(super) source: Site,
    pub(super) reached: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Node {
    pub(super) site: Site,
    pub(super) incoming: Vec<Incoming>,
    pub(super) outgoing: Vec<usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Graph<'a> {
    pub(super) edges: &'a [(Family, Edge)],
    pub(super) nodes: HashMap<Port, Node>,
    pub(super) external: HashMap<Port, Site>,
}

impl Checker {
    pub(super) fn scalar_graph<'a>(
        &mut self,
        reports: &'a Reports,
        ports: ports::Ports,
        span: Span,
        parts: usize,
    ) -> Result<(Graph<'a>, usize)> {
        if reports.index.edges.len() > MAX_EDGES
            || ports.len() > MAX_EDGES
            || !self.flow.spend(ports.len() + 1)
        {
            return Err(budget(span));
        }
        let mut parts = parts.min(MAX_EDGES);
        let mut graph = Graph {
            edges: &reports.index.edges,
            nodes: ports
                .into_iter()
                .map(|(port, site)| {
                    (
                        port,
                        Node {
                            site,
                            incoming: Vec::new(),
                            outgoing: Vec::new(),
                        },
                    )
                })
                .collect(),
            external: HashMap::new(),
        };
        for (position, (_, edge)) in graph.edges.iter().enumerate() {
            if !self.flow.spend(6) {
                return Err(budget(span));
            }
            let from = graph.nodes.get(&edge.from).map(|node| node.site);
            let to = graph.nodes.get(&edge.to).map(|node| node.site);
            let Some(site) = from.or(to) else {
                continue;
            };
            if from.is_some() && to.is_none() && edge.route != Route::Backedge {
                return Err(invalid(span));
            }
            if edge.route == Route::Backedge
                && !matches!(edge.from, Port::Restart { target, .. } if edge.to == Port::BlockEntry(target))
            {
                return Err(invalid(span));
            }
            let source = match from {
                Some(source) => source,
                None => {
                    self.scalar_external_site(&mut graph, edge.from, site.owner, span, &mut parts)?
                }
            };
            let target = match to {
                Some(target) => target,
                None => {
                    self.scalar_external_site(&mut graph, edge.to, site.owner, span, &mut parts)?
                }
            };
            if source.owner != target.owner {
                return Err(invalid(span));
            }
            let weight = usize::from(from.is_some()) + usize::from(to.is_some());
            parts = parts.checked_sub(weight).ok_or_else(|| budget(span))?;
            if let Some(node) = graph.nodes.get_mut(&edge.from) {
                node.outgoing.push(position);
            }
            if let Some(node) = graph.nodes.get_mut(&edge.to) {
                node.incoming.push(Incoming {
                    edge: position,
                    source,
                    reached: from.is_some(),
                });
            }
        }
        for node in graph.nodes.values_mut() {
            let len = node.incoming.len();
            if !self
                .flow
                .spend(len * (len.checked_ilog2().unwrap_or(0) as usize + 1) + 1)
            {
                return Err(budget(span));
            }
            node.incoming.sort_unstable_by_key(|input| {
                (
                    input.source.span.start,
                    input.source.span.end,
                    graph.edges[input.edge].1.from,
                    input.edge,
                )
            });
        }
        Ok((graph, parts))
    }

    pub(self) fn scalar_external_site(
        &mut self,
        graph: &mut Graph<'_>,
        port: Port,
        owner: usize,
        span: Span,
        parts: &mut usize,
    ) -> Result<Site> {
        if !self.flow.spend(2) {
            return Err(budget(span));
        }
        if let Some(site) = graph.external.get(&port) {
            return if site.owner == owner {
                Ok(*site)
            } else {
                Err(invalid(span))
            };
        }
        *parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
        let site = self.scalar_port_site(port, owner, span)?;
        graph.external.insert(port, site);
        Ok(site)
    }
}

pub(super) fn budget(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-graph budget exhausted", span)
}

pub(super) fn invalid(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-graph identity mismatch", span)
}

#[cfg(test)]
pub(super) mod tests;
