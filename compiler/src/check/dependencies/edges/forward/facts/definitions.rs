use super::super::effects::Effect;
use super::*;
use versions::{Definition, Version};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Location {
    pub(super) version: Version,
    pub(super) port: Port,
    pub(super) normal: Option<Port>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Definitions {
    pub(super) declarations: BTreeMap<hir::LocalId, Location>,
    pub(super) writes: BTreeMap<PointId, Location>,
}

impl Checker {
    pub(super) fn scalar_definitions(
        &mut self,
        reports: &Reports,
        versions: &versions::Versions<'_>,
        events: &events::Events<'_>,
        graph: &graph::Graph<'_>,
        span: Span,
        parts: usize,
    ) -> Result<(Definitions, usize)> {
        if versions.len() > MAX_EDGES
            || events.len() > MAX_EDGES
            || !self.flow.spend(versions.len() + events.len() + 1)
        {
            return Err(budget(span));
        }
        let mut parts = parts.min(MAX_EDGES);
        let mut definitions = Definitions::default();
        for (&local, slot) in versions {
            if local != slot.version.local
                || matches!(slot.version.definition, Definition::Write(_))
            {
                return Err(invalid(span));
            }
            let location =
                self.scalar_definition(reports, graph, slot.version, slot.source, span)?;
            self.insert_scalar_definition(&mut definitions, location, span, &mut parts)?;
        }
        for (&point, event) in events {
            let Some(version) = event.version else {
                continue;
            };
            if !matches!(version.definition, Definition::Bind(id) | Definition::Write(id) if id == point)
            {
                return Err(invalid(span));
            }
            let location =
                self.scalar_definition(reports, graph, version, Some(event.source), span)?;
            self.insert_scalar_definition(&mut definitions, location, span, &mut parts)?;
        }
        Ok((definitions, parts))
    }

    pub(self) fn scalar_definition(
        &mut self,
        reports: &Reports,
        graph: &graph::Graph<'_>,
        version: Version,
        source: Option<&(usize, Effect)>,
        span: Span,
    ) -> Result<Location> {
        if let Definition::Parameter(position) = version.definition {
            if !self.flow.spend(
                reports.signatures.params.len().checked_ilog2().unwrap_or(0) as usize
                    + reports
                        .signatures
                        .functions
                        .len()
                        .checked_ilog2()
                        .unwrap_or(0) as usize
                    + reports.entries.len().checked_ilog2().unwrap_or(0) as usize
                    + 5,
            ) {
                return Err(budget(span));
            }
            let function = version.owner.checked_sub(1).ok_or_else(|| invalid(span))?;
            let signature = reports
                .signatures
                .functions
                .get(&function)
                .ok_or_else(|| invalid(span))?;
            let port = Port::BlockEntry(signature.body);
            if source.is_some()
                || reports.signatures.params.get(&version.local) != Some(&(function, position))
                || signature.params.get(position) != Some(&version.local)
                || reports.entries.get(&version.owner).map(|entry| entry.0) != Some(signature.body)
                || graph.nodes.get(&port).is_none_or(|node| {
                    node.site.owner != version.owner || node.site.block != signature.body
                })
            {
                return Err(invalid(span));
            }
            return Ok(Location {
                version,
                port,
                normal: None,
            });
        }
        let (point, binding) = match version.definition {
            Definition::Bind(point) => (
                point,
                self.binding_effect(reports, point, version.owner, span)?,
            ),
            Definition::Write(point) => (
                point,
                self.write_effect(reports, point, version.owner, span)?,
            ),
            Definition::Parameter(_) => return Err(invalid(span)),
        };
        if !self
            .flow
            .spend(reports.effects.len().checked_ilog2().unwrap_or(0) as usize + 6)
        {
            return Err(budget(span));
        }
        let binding = binding.ok_or_else(|| invalid(span))?;
        let reported = reports.effects.get(&point).ok_or_else(|| invalid(span))?;
        let port = Port::Operation(point);
        let site = graph.nodes.get(&port).ok_or_else(|| invalid(span))?.site;
        let tracked = self.points.get(point).ok_or_else(|| invalid(span))?;
        if source.is_none_or(|source| !std::ptr::eq(source, reported))
            || binding.local != version.local
            || binding.storage != version.local
            || site.owner != version.owner
            || tracked.block != Some(site.block)
            || tracked.span != site.span
        {
            return Err(invalid(span));
        }
        let normal = if let Some(node) = graph.nodes.get(&Port::Normal(point)) {
            if node.site != site {
                return Err(invalid(span));
            }
            Some(Port::Normal(point))
        } else {
            None
        };
        Ok(Location {
            version,
            port,
            normal,
        })
    }

    pub(self) fn insert_scalar_definition(
        &mut self,
        definitions: &mut Definitions,
        location: Location,
        span: Span,
        parts: &mut usize,
    ) -> Result<()> {
        let (map, key) = match location.version.definition {
            Definition::Write(point) => (&mut definitions.writes, point),
            _ => (&mut definitions.declarations, location.version.local),
        };
        if !self
            .flow
            .spend(map.len().checked_ilog2().unwrap_or(0) as usize + 2)
        {
            return Err(budget(span));
        }
        *parts = parts
            .checked_sub(1 + usize::from(location.normal.is_some()))
            .ok_or_else(|| budget(span))?;
        if map.insert(key, location).is_some() {
            return Err(invalid(span));
        }
        Ok(())
    }
}

pub(super) fn budget(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-definition budget exhausted", span)
}

pub(super) fn invalid(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-definition identity mismatch", span)
}

#[cfg(test)]
mod tests;
