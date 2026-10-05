use super::*;
use crate::flow::Flow;

pub(super) fn collect(
    layout: &Layout,
    index: &index::Index<'_>,
    id: hir::BlockId,
    flow: &mut Flow,
    mut parts: usize,
    span: Span,
) -> Result<(Option<Vec<Sources>>, usize)> {
    let budget = || Diagnostic::unsupported("proof result-source budget exhausted", span);
    let invalid = || Diagnostic::unsupported("proof result-source identity mismatch", span);
    let slots = match layout {
        Layout::Stopped => return Err(invalid()),
        Layout::Unknown => None,
        Layout::Slots(slots) => {
            parts = parts.checked_sub(slots.len()).ok_or_else(budget)?;
            let mut sources = Vec::with_capacity(slots.len());
            for slot in slots {
                if !flow.spend(
                    (slot.field.as_ref().map_or(0, String::len) + 1)
                        * (index.len().checked_ilog2().unwrap_or(0) as usize + 3),
                ) {
                    return Err(budget());
                }
                let row = index.get(&(id, slot.field.as_deref()));
                let source = if slot.mutable
                    || !matches!(slot.shape, Shape::Scalar(_))
                    || row.is_some_and(|row| row.mutable)
                {
                    Sources::Unknown
                } else {
                    let values = row.map_or(&[][..], |row| row.values.as_slice());
                    parts = parts.checked_sub(values.len()).ok_or_else(budget)?;
                    if !flow.spend(values.len() + 1) {
                        return Err(budget());
                    }
                    Sources::Candidates(values.to_vec())
                };
                sources.push(source);
            }
            Some(sources)
        }
    };
    Ok((slots, parts))
}
