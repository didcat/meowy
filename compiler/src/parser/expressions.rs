use super::{ParseResult, Parser, bounded_tree, is_comparison, precedence};
use crate::ast::{Expr, ExprKind, Param, Span, StringPart};
use crate::diagnostic::Diagnostic;
use crate::lexer::TokenKind;

impl Parser {
    pub(crate) fn expr(
        &mut self,
        min: u8,
        condition: bool,
        multiline: bool,
        pipe: bool,
    ) -> ParseResult<Expr> {
        if self.depth >= 64 {
            return Err(Diagnostic::unsupported(
                "expression nesting beyond 64 levels",
                self.token().span,
            ));
        }
        self.depth += 1;
        let result = self.expr_inner(min, condition, multiline, pipe);
        self.depth -= 1;
        result
    }

    pub(crate) fn expr_inner(
        &mut self,
        min: u8,
        condition: bool,
        multiline: bool,
        pipe: bool,
    ) -> ParseResult<Expr> {
        if multiline {
            self.newlines();
        }
        let mut left = self.prefix(min, condition, multiline, pipe)?;
        loop {
            if !bounded_tree(&left) {
                return Err(Diagnostic::unsupported(
                    "syntax tree depth beyond 256 levels",
                    left.span,
                ));
            }
            if multiline {
                self.newlines();
            } else if self.token().kind == TokenKind::Newline {
                let save = self.pos;
                self.newlines();
                if !self.at(".") {
                    self.pos = save;
                    break;
                }
            }
            let start = left.span.start;
            if min <= 100 {
                if self.subtraction_suffix() {
                    left = self.type_subtraction(left)?;
                    continue;
                }
                if self.take("(") {
                    let args = self.arguments(")")?;
                    left = Expr {
                        kind: ExprKind::Call {
                            callee: Box::new(left),
                            args,
                        },
                        span: Span::new(start, self.end()),
                    };
                    continue;
                }
                if self.take("[") {
                    let index = self.expr(0, false, true, false)?;
                    self.need("]")?;
                    left = Expr {
                        kind: ExprKind::Index {
                            value: Box::new(left),
                            index: Box::new(index),
                        },
                        span: Span::new(start, self.end()),
                    };
                    continue;
                }
                if self.take(".") {
                    self.newlines();
                    let kind = if self.take("(") {
                        self.newlines();
                        let callee = self.expr(0, false, true, false)?;
                        let args = if self.take(",") {
                            self.arguments(")")?
                        } else {
                            self.need(")")?;
                            Vec::new()
                        };
                        ExprKind::Dispatch {
                            value: Box::new(left),
                            callee: Box::new(callee),
                            args,
                        }
                    } else if self.take("{") {
                        let block = self.block(None, true, self.tokens[self.pos - 1].span.start);
                        ExprKind::DispatchBlock {
                            value: Box::new(left),
                            block,
                        }
                    } else if self.at("&") || self.at("&!") || self.at("*") {
                        let op = self.bump().text;
                        self.newlines();
                        let name = self.name()?.text;
                        let field = Expr {
                            kind: ExprKind::Field {
                                value: Box::new(left),
                                name,
                            },
                            span: Span::new(start, self.end()),
                        };
                        ExprKind::Unary {
                            op,
                            value: Box::new(field),
                        }
                    } else {
                        ExprKind::Field {
                            value: Box::new(left),
                            name: self.name()?.text,
                        }
                    };
                    left = Expr {
                        kind,
                        span: Span::new(start, self.end()),
                    };
                    continue;
                }
                if self.at("<") {
                    if self.specialized_call() {
                        self.need("<")?;
                        let mut types = Vec::new();
                        loop {
                            self.newlines();
                            if self.token().kind == TokenKind::Int {
                                return Err(Diagnostic::unsupported(
                                    "value arguments in generic calls",
                                    self.token().span,
                                ));
                            }
                            if types.len() == 64 {
                                return Err(Diagnostic::unsupported(
                                    "explicit type arguments beyond 64 entries",
                                    self.token().span,
                                ));
                            }
                            types.push(self.type_atom()?);
                            self.newlines();
                            if self.take(">") {
                                break;
                            }
                            self.need(",")?;
                        }
                        left = Expr {
                            kind: ExprKind::Specialize {
                                value: Box::new(left),
                                types,
                            },
                            span: Span::new(start, self.end()),
                        };
                        continue;
                    }
                    let save = self.pos;
                    self.bump();
                    self.newlines();
                    if self.take(">") {
                        left = Expr {
                            kind: ExprKind::TypeQuery(Box::new(left)),
                            span: Span::new(start, self.end()),
                        };
                        continue;
                    }
                    self.pos = save;
                    if !condition || min <= 40 {
                        if let Ok(ty) = self.type_union() {
                            if self.at("(") {
                                if matches!(ty.kind, crate::ast::TypeKind::Union(_)) {
                                    return Err(Diagnostic::unsupported(
                                        "union suffixes in generic calls; use a named type argument",
                                        ty.span,
                                    ));
                                }
                                left = Expr {
                                    kind: ExprKind::Specialize {
                                        value: Box::new(left),
                                        types: vec![ty],
                                    },
                                    span: Span::new(start, self.end()),
                                };
                                continue;
                            }
                            if condition && is_comparison(&left) {
                                return Err(Diagnostic::new(
                                    "E004",
                                    "comparisons cannot be chained",
                                    ty.span,
                                ));
                            }
                            left = Expr {
                                kind: ExprKind::Ascribe {
                                    value: Box::new(left),
                                    ty,
                                    predicate: condition,
                                },
                                span: Span::new(start, self.end()),
                            };
                            continue;
                        }
                        self.pos = save;
                    }
                }
            }
            if pipe && self.at("|") {
                break;
            }
            let op = self.token().text.clone();
            let Some(level) = precedence(&op) else {
                break;
            };
            if level < min {
                break;
            }
            if level == 40 && is_comparison(&left) {
                return Err(Diagnostic::new(
                    "E004",
                    "comparisons cannot be chained",
                    self.token().span,
                ));
            }
            self.bump();
            self.newlines();
            let right = self.expr(level + 1, condition, multiline, pipe)?;
            left = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span: Span::new(start, self.end()),
            };
        }
        Ok(left)
    }

    pub(crate) fn prefix(
        &mut self,
        min: u8,
        condition: bool,
        multiline: bool,
        pipe: bool,
    ) -> ParseResult<Expr> {
        let token = self.bump();
        let start = token.span.start;
        let kind = match token.kind {
            TokenKind::Int => ExprKind::Int(token.text),
            TokenKind::Float => ExprKind::Float(token.text),
            TokenKind::String => ExprKind::String(self.string_parts(&token)?),
            TokenKind::Name => ExprKind::Name(token.text),
            _ => match token.text.as_str() {
                "(" => {
                    if let Some((params, label)) = self.function_head()? {
                        self.need("{")?;
                        ExprKind::Function {
                            params,
                            body: self.block(label, true, start),
                        }
                    } else {
                        let value = self.expr(0, condition, true, false)?;
                        self.need(")")?;
                        ExprKind::Group(Box::new(value))
                    }
                }
                "{" => ExprKind::Block(self.block(None, true, start)),
                "'" => {
                    let label = self.name()?.text;
                    if self.take("{") {
                        ExprKind::Block(self.block(Some(label), true, start))
                    } else {
                        ExprKind::Label(label)
                    }
                }
                "[" => ExprKind::List(self.list_values()?),
                "@" => {
                    let path = self.bump();
                    if path.kind != TokenKind::String {
                        return Err(Diagnostic::new(
                            "E004",
                            "module import requires a string literal",
                            path.span,
                        ));
                    }
                    let parts = self.string_parts(&path)?;
                    if parts
                        .iter()
                        .any(|part| matches!(part, StringPart::Value(_)))
                    {
                        return Err(Diagnostic::unsupported(
                            "interpolated module paths",
                            path.span,
                        ));
                    }
                    ExprKind::Import(
                        parts
                            .into_iter()
                            .filter_map(|part| {
                                if let StringPart::Text(text) = part {
                                    Some(text)
                                } else {
                                    None
                                }
                            })
                            .collect(),
                    )
                }
                "<" => {
                    self.pos -= 1;
                    ExprKind::TypeValue(self.type_union()?)
                }
                "!" if self.at("{") => {
                    return Err(Diagnostic::unsupported(
                        "unchecked blocks",
                        Span::new(start, self.token().span.end),
                    ));
                }
                "!" | "-" | "~" | "*" | "&" | "&!" | ">>" | "<<" => {
                    self.newlines();
                    let level = if matches!(token.text.as_str(), "&" | "&!" | "*") {
                        101
                    } else {
                        min.max(90)
                    };
                    let value = self.expr(level, condition, multiline, pipe)?;
                    ExprKind::Unary {
                        op: token.text,
                        value: Box::new(value),
                    }
                }
                ")" | "]" | "}" => {
                    return Err(Diagnostic::new(
                        "E002",
                        "unexpected closing delimiter",
                        token.span,
                    ));
                }
                _ => {
                    return Err(Diagnostic::new(
                        "E004",
                        "expected an expression",
                        token.span,
                    ));
                }
            },
        };
        Ok(Expr {
            kind,
            span: Span::new(start, self.end()),
        })
    }

    pub(crate) fn specialized_call(&self) -> bool {
        if self.tokens[self.pos + 1..]
            .iter()
            .find(|token| token.kind != TokenKind::Newline)
            .is_some_and(|token| token.text == ">")
        {
            return false;
        }
        let mut angles = 0usize;
        let mut parens = 0usize;
        let mut brackets = 0usize;
        for (offset, token) in self.tokens[self.pos..].iter().enumerate() {
            if token.kind == TokenKind::Eof {
                return false;
            }
            match token.text.as_str() {
                "(" => parens += 1,
                ")" if parens > 0 => parens -= 1,
                "[" => brackets += 1,
                "]" if brackets > 0 => brackets -= 1,
                "<" if parens == 0 && brackets == 0 => angles += 1,
                ">" | ">>" if parens == 0 && brackets == 0 => {
                    let count = token.text.len();
                    if count > angles {
                        return false;
                    }
                    angles -= count;
                    if angles == 0 {
                        return self
                            .tokens
                            .get(self.pos + offset + 1)
                            .is_some_and(|next| next.text == "(");
                    }
                }
                "," | "." | "&" | "&!" | "*" | "!" => {}
                _ if parens > 0 || brackets > 0 => {}
                _ if matches!(
                    token.kind,
                    TokenKind::Name | TokenKind::Int | TokenKind::Newline
                ) => {}
                _ => return false,
            }
        }
        false
    }

    pub(crate) fn function_head(&mut self) -> ParseResult<Option<(Vec<Param>, Option<String>)>> {
        let save = self.pos;
        self.newlines();
        let mut params = Vec::new();
        if !self.at(")") {
            loop {
                let name = match self.name() {
                    Ok(name) => name,
                    Err(_) => {
                        self.pos = save;
                        return Ok(None);
                    }
                };
                let ty = match self.type_union() {
                    Ok(ty) => ty,
                    Err(_) => {
                        self.pos = save;
                        return Ok(None);
                    }
                };
                params.push(Param {
                    name: name.text,
                    span: Span::new(name.span.start, ty.span.end),
                    ty,
                });
                self.newlines();
                if !self.take(",") {
                    break;
                }
                self.newlines();
                if self.at(")") {
                    break;
                }
            }
        }
        if !self.take(")") {
            self.pos = save;
            return Ok(None);
        }
        let label = if self.take("'") {
            Some(self.name()?.text)
        } else {
            None
        };
        if self.at("!") {
            return Err(Diagnostic::unsupported(
                "unchecked functions",
                self.token().span,
            ));
        }
        if !self.at("{") {
            self.pos = save;
            return Ok(None);
        }
        Ok(Some((params, label)))
    }

    pub(crate) fn arguments(&mut self, close: &str) -> ParseResult<Vec<Expr>> {
        let mut args = Vec::new();
        self.newlines();
        if self.take(close) {
            return Ok(args);
        }
        loop {
            args.push(self.expr(0, false, true, false)?);
            if self.take(close) {
                break;
            }
            self.need(",")?;
            self.newlines();
            if self.take(close) {
                break;
            }
        }
        Ok(args)
    }

    pub(crate) fn list_values(&mut self) -> ParseResult<Vec<Expr>> {
        let mut values = Vec::new();
        self.newlines();
        if self.take("]") {
            return Ok(values);
        }
        loop {
            values.push(self.expr(0, false, true, false)?);
            if self.at(":") || self.at(":=") {
                return Err(Diagnostic::unsupported(
                    "named list aliases",
                    self.token().span,
                ));
            }
            if self.take("]") {
                return Ok(values);
            }
            self.need(",")?;
            self.newlines();
            if self.take("]") {
                return Ok(values);
            }
        }
    }
}
