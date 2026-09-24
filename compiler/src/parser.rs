mod bounds;
mod expressions;
mod statements;
mod strings;
mod types;

pub(crate) use bounds::bounded_tree;

use crate::ast::{Block, Expr, ExprKind, Stmt};
use crate::diagnostic::Diagnostic;
use crate::lexer::{self, Token, TokenKind};

pub(crate) type ParseResult<T> = Result<T, Diagnostic>;

pub(crate) enum TreeNode<'a> {
    Expr(&'a Expr),
    Stmt(&'a Stmt),
}

pub fn parse(source: &str) -> Result<Block, Vec<Diagnostic>> {
    let parsed = parse_documented(source)?;
    if !parsed.docs.is_empty() {
        crate::documentation::Model::new(source, &parsed).map_err(|error| vec![error])?;
    }
    Ok(parsed.block)
}

pub struct Parsed {
    pub block: Block,
    pub docs: Vec<lexer::Documentation>,
    pub marks: Vec<(TokenKind, crate::ast::Span)>,
}

pub fn parse_documented(source: &str) -> Result<Parsed, Vec<Diagnostic>> {
    parse_documented_at(source, 0)
}

pub(crate) fn parse_documented_at(source: &str, base: usize) -> Result<Parsed, Vec<Diagnostic>> {
    if base.checked_add(source.len()).is_none() {
        return Err(vec![Diagnostic::unsupported(
            "source span budget exhausted",
            crate::ast::Span::default(),
        )]);
    }
    let mut tokens = lexer::lex(source).map_err(|mut errors| {
        for error in &mut errors {
            error.span.start += base;
            error.span.end += base;
        }
        errors
    })?;
    for token in &mut tokens {
        token.span.start += base;
        token.span.end += base;
    }
    let mut parser = Parser::new(tokens);
    if !parser.errors.is_empty() {
        return Err(parser.errors);
    }
    let block = parser.block(None, false, base);
    if parser.errors.is_empty() {
        parser.docs.sort_by_key(|doc| doc.open.start);
        parser.docs.dedup_by_key(|doc| doc.open.start);
        parser.marks.sort_by_key(|(_, span)| (span.end, span.start));
        parser.marks.dedup();
        Ok(Parsed {
            block,
            docs: parser.docs,
            marks: parser.marks,
        })
    } else {
        Err(parser.errors)
    }
}

pub(crate) struct Parser {
    pub(crate) tokens: Vec<Token>,
    pub(crate) pos: usize,
    pub(crate) depth: usize,
    pub(crate) errors: Vec<Diagnostic>,
    pub(crate) docs: Vec<lexer::Documentation>,
    pub(crate) marks: Vec<(TokenKind, crate::ast::Span)>,
}

impl Parser {
    pub(crate) fn new(tokens: Vec<Token>) -> Self {
        let docs = tokens.iter().filter_map(Token::documentation).collect();
        let marks = tokens
            .iter()
            .filter(|token| {
                !matches!(
                    token.kind,
                    TokenKind::Space | TokenKind::Comment | TokenKind::Doc { .. } | TokenKind::Eof
                )
            })
            .map(|token| (token.kind, token.span))
            .collect();
        Self {
            tokens: tokens
                .into_iter()
                .filter(|token| {
                    !matches!(
                        token.kind,
                        TokenKind::Space | TokenKind::Comment | TokenKind::Doc { .. }
                    )
                })
                .collect(),
            pos: 0,
            depth: 0,
            errors: Vec::new(),
            docs,
            marks,
        }
    }

    pub(crate) fn token(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    pub(crate) fn at(&self, text: &str) -> bool {
        self.token().text == text
    }

    pub(crate) fn eof(&self) -> bool {
        self.token().kind == TokenKind::Eof
    }

    pub(crate) fn bump(&mut self) -> Token {
        let token = self.token().clone();
        if !self.eof() {
            self.pos += 1;
        }
        token
    }

    pub(crate) fn take(&mut self, text: &str) -> bool {
        if self.at(text) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(crate) fn need(&mut self, text: &str) -> ParseResult<Token> {
        if self.at(text) {
            Ok(self.bump())
        } else {
            let code = if matches!(text, ")" | "]" | "}" | ">") {
                "E002"
            } else {
                "E004"
            };
            Err(Diagnostic::new(
                code,
                format!("expected `{text}`, found {}", self.description()),
                self.token().span,
            ))
        }
    }

    pub(crate) fn description(&self) -> String {
        if self.eof() {
            "end of file".to_owned()
        } else if self.token().kind == TokenKind::Newline {
            "newline".to_owned()
        } else {
            format!("`{}`", self.token().text)
        }
    }

    pub(crate) fn name(&mut self) -> ParseResult<Token> {
        if self.token().kind == TokenKind::Name {
            Ok(self.bump())
        } else {
            Err(Diagnostic::new(
                "E004",
                "expected a name",
                self.token().span,
            ))
        }
    }

    pub(crate) fn newlines(&mut self) {
        while self.token().kind == TokenKind::Newline {
            self.bump();
        }
    }

    pub(crate) fn separators(&mut self) {
        while self.token().kind == TokenKind::Newline || self.at(";") {
            self.bump();
        }
    }

    pub(crate) fn end(&self) -> usize {
        if self.pos == 0 {
            self.token().span.start
        } else {
            self.tokens[self.pos - 1].span.end
        }
    }

    pub(crate) fn terminator(&self) -> bool {
        self.eof() || self.at(";") || self.at("}") || self.token().kind == TokenKind::Newline
    }
}

pub(crate) fn precedence(op: &str) -> Option<u8> {
    match op {
        "*" | "/" | "%" => Some(80),
        "+" | "-" => Some(70),
        "<" | "<=" | ">" | ">=" => Some(40),
        "==" | "!=" => Some(30),
        "&&" => Some(20),
        "||" => Some(10),
        _ => None,
    }
}

pub(crate) fn is_comparison(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::Binary { op, .. } if matches!(op.as_str(), "<" | "<=" | ">" | ">="))
        || matches!(
            expr.kind,
            ExprKind::Ascribe {
                predicate: true,
                ..
            }
        )
}

#[cfg(test)]
mod tests;
