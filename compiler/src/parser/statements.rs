use super::{ParseResult, Parser};
use crate::ast::{Block, Span, Stmt, StmtKind, TypeExpr, TypeKind};
use crate::diagnostic::Diagnostic;
use crate::lexer::TokenKind;

impl Parser {
    pub(crate) fn block(&mut self, label: Option<String>, closed: bool, start: usize) -> Block {
        let mut stmts = Vec::new();
        self.separators();
        while !self.eof() && !(closed && self.at("}")) {
            let before = self.pos;
            match self.statement() {
                Ok(stmt) => {
                    stmts.push(stmt);
                    if !self.terminator() {
                        self.errors.push(Diagnostic::new(
                            "E004",
                            "expected a newline or `;` between statements",
                            self.token().span,
                        ));
                        self.recover(closed);
                    }
                }
                Err(error) => {
                    self.errors.push(error);
                    self.recover(closed);
                }
            }
            if self.pos == before && !self.eof() && !(closed && self.at("}")) {
                self.bump();
            }
            self.separators();
        }
        if closed && let Err(error) = self.need("}") {
            self.errors.push(error);
        }
        Block {
            label,
            stmts,
            span: Span::new(start, self.end()),
        }
    }

    pub(crate) fn recover(&mut self, closed: bool) {
        while !self.eof()
            && !self.at(";")
            && self.token().kind != TokenKind::Newline
            && !(closed && self.at("}"))
        {
            self.bump();
        }
    }

    pub(crate) fn statement(&mut self) -> ParseResult<Stmt> {
        if self.depth >= 64 {
            return Err(Diagnostic::unsupported(
                "statement nesting beyond 64 levels",
                self.token().span,
            ));
        }
        self.depth += 1;
        let result = self.statement_inner();
        self.depth -= 1;
        result
    }

    pub(crate) fn statement_inner(&mut self) -> ParseResult<Stmt> {
        let start = self.token().span.start;
        let kind = if self.take("|") {
            let condition = self.expr(0, false)?;
            self.need("|")?;
            self.newlines();
            let body = self.statement()?;
            StmtKind::Match {
                arms: vec![(Some(condition), Box::new(body))],
            }
        } else if self.take("->") {
            self.emission(None)?
        } else if self.at("'")
            && self
                .tokens
                .get(self.pos + 2)
                .is_some_and(|token| token.text == "->")
        {
            self.bump();
            let label = self.name()?.text;
            self.need("->")?;
            self.emission(Some(label))?
        } else if self.at("<") {
            self.type_alias(false)?
        } else if let Some((name, ty, mutable)) = self.binding_head()? {
            self.newlines();
            StmtKind::Bind {
                name,
                ty,
                mutable,
                value: self.expr(0, false)?,
            }
        } else {
            let save = self.pos;
            let forward = if self.token().kind == TokenKind::Name {
                let name = self.bump().text;
                match self.type_union() {
                    Ok(ty) if self.terminator() && matches!(ty.kind, TypeKind::Function { .. }) => {
                        Some(StmtKind::Forward { name, ty })
                    }
                    _ => None,
                }
            } else {
                None
            };
            if let Some(kind) = forward {
                kind
            } else {
                self.pos = save;
                let target = self.expr(0, false)?;
                if self.take("=") {
                    self.newlines();
                    StmtKind::Assign {
                        target,
                        value: self.expr(0, false)?,
                    }
                } else {
                    StmtKind::Expr(target)
                }
            }
        };
        Ok(Stmt {
            kind,
            span: Span::new(start, self.end()),
        })
    }

    pub(crate) fn type_alias(&mut self, exported: bool) -> ParseResult<StmtKind> {
        let alias = self.type_union()?;
        self.need(":")?;
        self.newlines();
        let TypeKind::Name(name) = alias.kind else {
            return Err(Diagnostic::unsupported(
                "generic or computed type alias names",
                alias.span,
            ));
        };
        if exported && name.contains('.') {
            return Err(Diagnostic::unsupported(
                "qualified exported type names",
                alias.span,
            ));
        }
        let ty = if self.at("<") {
            self.type_union()?
        } else {
            let value = self.expr(0, false)?;
            TypeExpr {
                span: value.span,
                kind: TypeKind::Computed(Box::new(value)),
            }
        };
        Ok(StmtKind::TypeAlias { name, ty, exported })
    }

    pub(crate) fn binding_head(&mut self) -> ParseResult<Option<(String, Option<TypeExpr>, bool)>> {
        if self.token().kind != TokenKind::Name {
            return Ok(None);
        }
        let save = self.pos;
        let name = self.bump().text;
        let ty = if self.at("<") {
            match self.type_union() {
                Ok(ty) => Some(ty),
                Err(error) if error.code == "B001" && self.annotation_binding(save + 1) => {
                    return Err(error);
                }
                Err(_) => {
                    self.pos = save;
                    return Ok(None);
                }
            }
        } else {
            None
        };
        if self.at(":") || self.at(":=") {
            let mutable = self.bump().text == ":=";
            Ok(Some((name, ty, mutable)))
        } else {
            self.pos = save;
            Ok(None)
        }
    }

    pub(crate) fn emission(&mut self, label: Option<String>) -> ParseResult<StmtKind> {
        self.newlines();
        if self.at("<") && self.annotation_binding(self.pos) {
            if label.is_some() {
                return Err(Diagnostic::unsupported(
                    "labeled type exports",
                    self.token().span,
                ));
            }
            return self.type_alias(true);
        }
        let (name, ty, mutable) = match self.binding_head()? {
            Some((name, ty, mutable)) => (Some(name), ty, mutable),
            None => (None, None, false),
        };
        self.newlines();
        let value = self.expr(0, false)?;
        Ok(StmtKind::Emit {
            label,
            name,
            ty,
            mutable,
            value,
        })
    }

    pub(crate) fn annotation_binding(&self, start: usize) -> bool {
        let mut depth = 0usize;
        for token in &self.tokens[start..] {
            match token.text.as_str() {
                "<" => depth += 1,
                ">" | ">>" if token.text.len() <= depth => depth -= token.text.len(),
                "!" if depth == 0 => {}
                ":" | ":=" if depth == 0 => return true,
                ";" if depth == 0 => return false,
                _ if depth == 0 => return false,
                _ => {}
            }
        }
        false
    }
}
