use super::{ParseResult, Parser};
use crate::ast::{Expr, ExprKind, Span, TypeExpr, TypeKind};
use crate::diagnostic::Diagnostic;

impl Parser {
    pub(crate) fn type_union(&mut self) -> ParseResult<TypeExpr> {
        let mut members = vec![self.type_bracket()?];
        while self.at("<") {
            let save = self.pos;
            match self.type_bracket() {
                Ok(ty) => members.push(ty),
                Err(_) => {
                    self.pos = save;
                    break;
                }
            }
        }
        let mut ty = if members.len() == 1 {
            members.remove(0)
        } else {
            let span = Span::new(members[0].span.start, members.last().unwrap().span.end);
            TypeExpr {
                kind: TypeKind::Union(members),
                span,
            }
        };
        let mut count = 0;
        while self.subtraction_suffix() {
            count += 1;
            if count > 64 {
                return Err(Diagnostic::unsupported(
                    "type subtraction chain beyond 64 operations",
                    self.token().span,
                ));
            }
            let span = ty.span;
            let value = self.type_subtraction(Expr {
                kind: ExprKind::TypeValue(ty),
                span,
            })?;
            ty = TypeExpr {
                span: value.span,
                kind: TypeKind::Computed(Box::new(value)),
            };
        }
        Ok(ty)
    }

    pub(crate) fn subtraction_suffix(&self) -> bool {
        self.at("!")
            && self.tokens[self.pos + 1..]
                .iter()
                .find(|token| token.kind != crate::lexer::TokenKind::Newline)
                .is_some_and(|token| token.text == "<")
    }

    pub(crate) fn type_subtraction(&mut self, left: Expr) -> ParseResult<Expr> {
        self.need("!")?;
        self.newlines();
        let ty = self.type_bracket()?;
        let span = Span::new(left.span.start, ty.span.end);
        if self.at("<") {
            let save = self.pos;
            let next = self.type_bracket();
            self.pos = save;
            if next.is_ok() {
                return Err(Diagnostic::unsupported(
                    "union suffixes after type subtraction; use a named union operand",
                    self.token().span,
                ));
            }
        }
        Ok(Expr {
            span,
            kind: ExprKind::Binary {
                op: "!".into(),
                left: Box::new(left),
                right: Box::new(Expr {
                    span: ty.span,
                    kind: ExprKind::TypeValue(ty),
                }),
            },
        })
    }

    pub(crate) fn type_bracket(&mut self) -> ParseResult<TypeExpr> {
        if self.depth >= 64 {
            return Err(Diagnostic::unsupported(
                "type nesting beyond 64 levels",
                self.token().span,
            ));
        }
        self.depth += 1;
        let result = self.type_bracket_inner();
        self.depth -= 1;
        result
    }

    pub(crate) fn type_bracket_inner(&mut self) -> ParseResult<TypeExpr> {
        let start = self.need("<")?.span.start;
        self.newlines();
        if self.at(":") {
            return Err(Diagnostic::unsupported(
                "generic type binders",
                self.token().span,
            ));
        }
        let mut ty = self.type_atom()?;
        self.newlines();
        self.need(">")?;
        ty.span = Span::new(start, self.end());
        Ok(ty)
    }

    pub(crate) fn type_atom(&mut self) -> ParseResult<TypeExpr> {
        if self.depth >= 64 {
            return Err(Diagnostic::unsupported(
                "type nesting beyond 64 levels",
                self.token().span,
            ));
        }
        self.depth += 1;
        let result = self.type_atom_inner();
        self.depth -= 1;
        result
    }

    pub(crate) fn type_atom_inner(&mut self) -> ParseResult<TypeExpr> {
        self.newlines();
        let start = self.token().span.start;
        let kind = if self.take("{") {
            let mut primary = None;
            let mut fields = Vec::new();
            self.separators();
            while !self.at("}") && !self.eof() {
                if self.take("->") {
                    self.newlines();
                    if primary.is_some() {
                        return Err(Diagnostic::new(
                            "E203",
                            "duplicate record primary type",
                            self.token().span,
                        ));
                    }
                    primary = Some(Box::new(self.type_union()?));
                } else {
                    let name = self.name()?.text;
                    self.newlines();
                    let ty = self.type_union()?;
                    let mutable = self.take(":=");
                    fields.push((name, ty, mutable));
                }
                if !self.at("}") && !self.terminator() {
                    return Err(Diagnostic::new(
                        "E004",
                        "expected a separator between record fields",
                        self.token().span,
                    ));
                }
                self.separators();
            }
            self.need("}")?;
            TypeKind::Record { primary, fields }
        } else if self.at("&") || self.at("&!") {
            let mutable = self.bump().text == "&!";
            TypeKind::Reference {
                value: Box::new(self.type_atom()?),
                mutable,
            }
        } else if self.at("*") {
            return Err(Diagnostic::unsupported(
                "raw pointer types",
                self.token().span,
            ));
        } else if self.take("(") {
            let save = self.pos;
            let parsed = self.function_type_params();
            if let Ok(params) = parsed {
                self.newlines();
                if self.take("->") {
                    let result = self.type_atom()?;
                    TypeKind::Function {
                        params,
                        result: Box::new(result),
                    }
                } else {
                    self.pos = save;
                    let value = self.expr(0, false, true, false)?;
                    self.need(")")?;
                    TypeKind::Computed(Box::new(value))
                }
            } else {
                self.pos = save;
                let value = self.expr(0, false, true, false)?;
                self.need(")")?;
                TypeKind::Computed(Box::new(value))
            }
        } else {
            let mut name = self.name()?.text;
            while self.take(".") {
                name.push('.');
                name.push_str(&self.name()?.text);
            }
            if self.at("<") {
                return Err(Diagnostic::unsupported(
                    "generic type arguments",
                    self.token().span,
                ));
            }
            TypeKind::Name(name)
        };
        let mut ty = TypeExpr {
            kind,
            span: Span::new(start, self.end()),
        };
        while self.take("[") {
            self.newlines();
            let size = if self.at("]") {
                None
            } else {
                Some(Box::new(self.expr(0, false, true, false)?))
            };
            self.need("]")?;
            ty = TypeExpr {
                kind: TypeKind::List {
                    element: Box::new(ty),
                    size,
                },
                span: Span::new(start, self.end()),
            };
        }
        Ok(ty)
    }

    pub(crate) fn function_type_params(&mut self) -> ParseResult<Vec<TypeExpr>> {
        let mut params = Vec::new();
        self.newlines();
        if self.take(")") {
            return Ok(params);
        }
        loop {
            params.push(if self.at("<") {
                self.type_union()?
            } else {
                self.type_atom()?
            });
            self.newlines();
            if self.take(")") {
                break;
            }
            self.need(",")?;
            self.newlines();
        }
        Ok(params)
    }
}
