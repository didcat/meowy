use super::Output;
use crate::ast::{Stmt, StmtKind};
use crate::check::{Checker, Result, Spec, Value};
use crate::diagnostic::Diagnostic;

impl Checker {
    pub(crate) fn type_statements(&mut self, stmts: &[Stmt], output: &mut Output) -> Result<()> {
        for stmt in stmts {
            self.type_statement(stmt, output)?;
        }
        Ok(())
    }

    pub(crate) fn type_statement(&mut self, stmt: &Stmt, output: &mut Output) -> Result<()> {
        self.type_work.as_mut().unwrap().spend(stmt.span)?;
        match &stmt.kind {
            StmtKind::Bind {
                name,
                ty,
                mutable: false,
                value,
            } => {
                let value = self.type_binding(value, ty.as_ref())?;
                self.declare(name, value, stmt.span)?;
            }
            StmtKind::TypeAlias {
                name,
                ty,
                exported: false,
            } => {
                self.declare_type(name, ty, false, stmt.span)?;
                let spec = &self.scopes.last().unwrap().types[name];
                let work = self.type_work.as_mut().unwrap();
                match spec {
                    Spec::Data(ty) => work.materialize(ty, stmt.span)?,
                    Spec::Function { params, result } => {
                        for ty in params.iter().chain(std::iter::once(result)) {
                            work.materialize(ty, stmt.span)?;
                        }
                    }
                }
            }
            StmtKind::Emit {
                label: None,
                name: None,
                ty: None,
                mutable: false,
                value,
            } => {
                let value = match output.ty.as_ref() {
                    Some(ty) => self.scalar_emission(value, ty)?,
                    None => Value::Type(self.type_value(value)?),
                };
                if output.value.replace(value).is_some() {
                    return Err(Self::error(
                        "E205",
                        if output.ty.is_some() {
                            "required scalar primary may be emitted twice"
                        } else {
                            "computed type primary may be emitted twice"
                        },
                        stmt.span,
                    ));
                }
            }
            StmtKind::Match { arms } => self.type_match(arms, stmt.span, output)?,
            StmtKind::Expr(value) => return Err(self.type_unavailable(value)?),
            _ => {
                return Err(Diagnostic::unsupported(
                    if output.ty.is_some() {
                        "required scalar block statement"
                    } else {
                        "computed type block statement"
                    },
                    stmt.span,
                ));
            }
        }
        self.doc_stage(stmt.span.start)?;
        Ok(())
    }
}
