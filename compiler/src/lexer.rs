use crate::ast::Span;
use crate::diagnostic::Diagnostic;

mod comments;
pub use comments::Documentation;
pub(crate) use comments::scan as scan_comment;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Name,
    Int,
    Float,
    String,
    Symbol,
    Space,
    Newline,
    Comment,
    Doc { module: bool, bars: usize },
    Eof,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub span: Span,
}

pub fn lex(source: &str) -> Result<Vec<Token>, Vec<Diagnostic>> {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();
    let bytes = source.as_bytes();
    let mut pos = 0;
    while pos < bytes.len() {
        let start = pos;
        let kind = match bytes[pos] {
            b' ' | b'\t' => {
                pos += 1;
                while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t') {
                    pos += 1;
                }
                TokenKind::Space
            }
            b'\n' => {
                pos += 1;
                TokenKind::Newline
            }
            b'\r' if bytes.get(pos + 1) == Some(&b'\n') => {
                pos += 2;
                TokenKind::Newline
            }
            b'#' => {
                let (end, kind) = comments::scan(source, start, &mut errors);
                pos = end;
                kind
            }
            b'"' => {
                pos = scan_string(source, pos, 0, &mut errors);
                TokenKind::String
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                pos += 1;
                while pos < bytes.len()
                    && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_')
                {
                    pos += 1;
                }
                TokenKind::Name
            }
            b'0'..=b'9' => {
                let radix = bytes.get(pos) == Some(&b'0')
                    && matches!(bytes.get(pos + 1), Some(b'x' | b'X' | b'b' | b'B'));
                pos += 1;
                while pos < bytes.len() {
                    let byte = bytes[pos];
                    if byte.is_ascii_alphanumeric()
                        || byte == b'_'
                        || (byte == b'.' && !matches!(bytes.get(pos + 1), Some(b'(' | b'{')))
                        || (!radix
                            && matches!(byte, b'+' | b'-')
                            && matches!(bytes.get(pos - 1), Some(b'e' | b'E')))
                    {
                        pos += 1;
                    } else {
                        break;
                    }
                }
                match number_kind(&source[start..pos]) {
                    Some(kind) => kind,
                    None => {
                        errors.push(Diagnostic::new(
                            "E001",
                            "malformed numeric literal",
                            Span::new(start, pos),
                        ));
                        TokenKind::Int
                    }
                }
            }
            byte if b"{}()[]<>:;,.|+-*/%&^!~=@'$".contains(&byte) => {
                pos += 1;
                if pos < bytes.len()
                    && matches!(
                        &bytes[start..pos + 1],
                        b"->"
                            | b":="
                            | b"&&"
                            | b"||"
                            | b"=="
                            | b"!="
                            | b"<="
                            | b">="
                            | b"&!"
                            | b">>"
                            | b"<<"
                    )
                {
                    pos += 1;
                }
                TokenKind::Symbol
            }
            _ => {
                pos += source[pos..].chars().next().map_or(1, char::len_utf8);
                errors.push(Diagnostic::new(
                    "E001",
                    "invalid source token",
                    Span::new(start, pos),
                ));
                TokenKind::Symbol
            }
        };
        tokens.push(Token {
            kind,
            text: source[start..pos].to_owned(),
            span: Span::new(start, pos),
        });
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        text: String::new(),
        span: Span::new(pos, pos),
    });
    if errors.is_empty() {
        Ok(tokens)
    } else {
        Err(errors)
    }
}

pub(crate) fn scan_string(
    source: &str,
    start: usize,
    depth: usize,
    errors: &mut Vec<Diagnostic>,
) -> usize {
    let bytes = source.as_bytes();
    if depth >= 128 {
        errors.push(Diagnostic::unsupported(
            "string nesting beyond 128 levels",
            Span::new(start, start + 1),
        ));
        return bytes.len();
    }
    let mut pos = start + 1;
    while pos < bytes.len() {
        match bytes[pos] {
            b'"' => return pos + 1,
            b'\\' => {
                let escape = pos;
                pos += 1;
                if let Some(byte) = bytes.get(pos) {
                    if !b"nrt0\"\\{}".contains(byte) {
                        let end = pos + source[pos..].chars().next().map_or(1, char::len_utf8);
                        errors.push(Diagnostic::new(
                            "E003",
                            "unsupported string escape",
                            Span::new(escape, end),
                        ));
                        pos = end;
                    } else {
                        pos += 1;
                    }
                }
            }
            b'{' => pos = scan_interpolation(source, pos, depth + 1, errors),
            b'}' => {
                errors.push(Diagnostic::new(
                    "E001",
                    "escape a literal closing brace as \\}",
                    Span::new(pos, pos + 1),
                ));
                pos += 1;
            }
            _ => pos += 1,
        }
    }
    errors.push(Diagnostic::new(
        "E002",
        "unclosed string",
        Span::new(start, bytes.len()),
    ));
    bytes.len()
}

pub(crate) fn scan_interpolation(
    source: &str,
    start: usize,
    depth: usize,
    errors: &mut Vec<Diagnostic>,
) -> usize {
    let bytes = source.as_bytes();
    let mut braces = 1usize;
    let mut pos = start + 1;
    while pos < bytes.len() {
        match bytes[pos] {
            b'"' => pos = scan_string(source, pos, depth, errors),
            b'#' => {
                pos = comments::scan(source, pos, errors).0;
            }
            b'{' => {
                braces += 1;
                pos += 1;
            }
            b'}' => {
                braces -= 1;
                pos += 1;
                if braces == 0 {
                    return pos;
                }
            }
            _ => pos += 1,
        }
    }
    errors.push(Diagnostic::new(
        "E002",
        "unclosed string interpolation",
        Span::new(start, pos),
    ));
    pos
}

pub(crate) fn number_kind(text: &str) -> Option<TokenKind> {
    if text.starts_with("0x") || text.starts_with("0X") {
        return digits(&text[2..], 16).then_some(TokenKind::Int);
    }
    if text.starts_with("0b") || text.starts_with("0B") {
        return digits(&text[2..], 2).then_some(TokenKind::Int);
    }
    let mut parts = text.split(['e', 'E']);
    let mantissa = parts.next()?;
    let exponent = parts.next();
    if parts.next().is_some() {
        return None;
    }
    if let Some(exp) = exponent
        && !digits(exp.strip_prefix(['+', '-']).unwrap_or(exp), 10)
    {
        return None;
    }
    let mut halves = mantissa.split('.');
    if !digits(halves.next()?, 10) {
        return None;
    }
    let fraction = halves.next();
    if halves.next().is_some() {
        return None;
    }
    if let Some(frac) = fraction
        && !digits(frac, 10)
    {
        return None;
    }
    Some(if fraction.is_some() || exponent.is_some() {
        TokenKind::Float
    } else {
        TokenKind::Int
    })
}

pub(crate) fn digits(text: &str, radix: u32) -> bool {
    let bytes = text.as_bytes();
    !bytes.is_empty()
        && bytes.iter().enumerate().all(|(pos, byte)| {
            if *byte == b'_' {
                pos > 0
                    && pos + 1 < bytes.len()
                    && (bytes[pos - 1] as char).is_digit(radix)
                    && (bytes[pos + 1] as char).is_digit(radix)
            } else {
                (*byte as char).is_digit(radix)
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(crate) fn retains_every_source_byte() {
        let source = "x # note\ninside # :\t\"é\\n\"\r\n->x";
        let tokens = lex(source).unwrap();
        assert_eq!(
            tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<String>(),
            source
        );
        for token in tokens {
            assert_eq!(&source[token.span.start..token.span.end], token.text);
        }
    }

    #[test]
    pub(crate) fn validates_number_grammar() {
        for text in [
            "42",
            "1_024",
            "0Xff",
            "0b10_10",
            "01",
            "3.5",
            "1e-3",
            "1_0.2_0e+1_0",
        ] {
            assert!(lex(text).is_ok(), "{text}");
        }
        for text in ["1.", "1__0", "0x_ff", "0b102", "12cat", "1e+", "1._2", "0x"] {
            assert_eq!(lex(text).unwrap_err()[0].code, "E001", "{text}");
        }
        assert_eq!(lex("1.(f)").unwrap()[0].text, "1");
    }

    #[test]
    pub(crate) fn diagnoses_invalid_whitespace_and_escapes() {
        for source in ["x\ry", "\u{feff}x", "x\u{a0}y", "café"] {
            assert_eq!(lex(source).unwrap_err()[0].code, "E001");
        }
        assert_eq!(lex("\"\\q\"").unwrap_err()[0].code, "E003");
        assert_eq!(lex("# open").unwrap_err()[0].code, "E002");
        assert_eq!(lex("\"open").unwrap_err()[0].code, "E002");
    }

    #[test]
    pub(crate) fn nested_interpolation_strings_remain_one_token() {
        let source = "\"outer {f(\"inner {2}\")} end\"";
        let tokens = lex(source).unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, source);
    }
}
