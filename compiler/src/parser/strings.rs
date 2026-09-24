use super::{ParseResult, Parser};
use crate::ast::StringPart;
use crate::diagnostic::Diagnostic;
use crate::lexer::{self, Token};

impl Parser {
    pub(crate) fn string_parts(&mut self, token: &Token) -> ParseResult<Vec<StringPart>> {
        let text = &token.text;
        let bytes = text.as_bytes();
        let mut pos = 1;
        let mut value = String::new();
        let mut parts = Vec::new();
        while pos + 1 < bytes.len() {
            match bytes[pos] {
                b'\\' => {
                    pos += 1;
                    value.push(match bytes[pos] {
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'0' => '\0',
                        byte => byte as char,
                    });
                    pos += 1;
                }
                b'{' => {
                    if !value.is_empty() {
                        parts.push(StringPart::Text(std::mem::take(&mut value)));
                    }
                    let mut errors = Vec::new();
                    let end = lexer::scan_interpolation(text, pos, 0, &mut errors);
                    if let Some(mut error) = errors.into_iter().next() {
                        error.span.start += token.span.start;
                        error.span.end += token.span.start;
                        return Err(error);
                    }
                    let offset = token.span.start + pos + 1;
                    let tokens = lexer::lex(&text[pos + 1..end - 1]).map_err(|mut errors| {
                        let mut error = errors.remove(0);
                        error.span.start += offset;
                        error.span.end += offset;
                        error
                    })?;
                    let tokens = tokens
                        .into_iter()
                        .map(|mut token| {
                            token.span.start += offset;
                            token.span.end += offset;
                            token
                        })
                        .collect();
                    let mut parser = Parser::new(tokens);
                    if !parser.errors.is_empty() {
                        return Err(parser.errors.remove(0));
                    }
                    parser.depth = self.depth;
                    let expression = parser.expr(0, true)?;
                    if !parser.errors.is_empty() {
                        return Err(parser.errors.remove(0));
                    }
                    if !parser.eof() {
                        return Err(Diagnostic::new(
                            "E004",
                            "expected one interpolation expression",
                            parser.token().span,
                        ));
                    }
                    parts.push(StringPart::Value(expression));
                    self.docs.extend(parser.docs);
                    self.marks.extend(parser.marks);
                    pos = end;
                }
                _ => {
                    let character = text[pos..].chars().next().unwrap();
                    value.push(character);
                    pos += character.len_utf8();
                }
            }
        }
        if !value.is_empty() || parts.is_empty() {
            parts.push(StringPart::Text(value));
        }
        Ok(parts)
    }
}
