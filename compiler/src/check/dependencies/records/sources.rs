use super::{Checker, Diagnostic, Expr, ExprKind, Origins, Result, Type};

impl Checker {
    pub(crate) fn record_source_origins(
        &mut self,
        value: &Expr,
        path: &[usize],
    ) -> Result<Origins> {
        let [index] = path else {
            return Ok(Origins::default());
        };
        Ok(match &value.kind {
            ExprKind::Local(_) => self.field_origins(value, *index),
            ExprKind::Block(block) => {
                if !self.flow.spend(self.proofs.aliases.len()) {
                    return Err(Diagnostic::unsupported(
                        "proof record origin budget exhausted",
                        value.span,
                    ));
                }
                let Type::Record { fields, .. } = &value.ty else {
                    return Ok(Origins::default());
                };
                self.proofs
                    .aliases
                    .values()
                    .find(|alias| alias.target == block.id && alias.field == fields[*index].name)
                    .and_then(|alias| self.pointees.get(&alias.root))
                    .cloned()
                    .unwrap_or_default()
            }
            _ => Origins::default(),
        })
    }
}
