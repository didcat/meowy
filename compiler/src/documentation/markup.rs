use super::{Entry, Result};
use crate::{ast::Span, diagnostic::Diagnostic};
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use std::ops::Range;

pub(crate) fn value_path(target: &str) -> Option<Vec<String>> {
    use crate::ast::{ExprKind, StmtKind};
    if target.len() > 1024 || target.contains(['#', ';', '\n', '\r']) {
        return None;
    }
    let parsed = crate::parser::parse_documented(target).ok()?;
    let [stmt] = parsed.block.stmts.as_slice() else {
        return None;
    };
    let StmtKind::Expr(expr) = &stmt.kind else {
        return None;
    };
    let mut expr = expr;
    let mut names = Vec::new();
    loop {
        match &expr.kind {
            ExprKind::Field { value, name } => {
                names.push(name.clone());
                expr = value;
            }
            ExprKind::Group(value) => expr = value,
            _ => {
                names.push(expr.spelling()?.into());
                break;
            }
        }
    }
    names.reverse();
    Some(names)
}

#[derive(Clone, Debug)]
pub(crate) struct Link {
    pub(crate) target: String,
    pub(crate) label: String,
    pub(crate) range: Range<usize>,
    pub(crate) span: Span,
    pub(crate) resolved: Option<usize>,
}
#[derive(Clone, Debug)]
pub(crate) enum ExampleMode {
    Check,
    Run,
    Reject(String),
}
#[derive(Clone, Debug)]
pub(crate) struct Example {
    pub(crate) mode: ExampleMode,
    pub(crate) source: String,
    pub(crate) expected: Option<String>,
    pub(crate) span: Span,
    pub(crate) ran: bool,
}

pub(crate) fn at(entry: &Entry, range: Range<usize>) -> Span {
    Span::new(
        entry.map[range.start.min(entry.text.len())],
        entry.map[range.end.min(entry.text.len())],
    )
}

pub(crate) fn analyze(entry: &mut Entry) -> Result<()> {
    let mut blocked = Vec::new();
    let mut code: Option<(String, String, usize)> = None;
    let mut last_run = None;
    for (event, range) in Parser::new(&entry.text).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let info = match kind {
                    CodeBlockKind::Fenced(info) => info.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                code = Some((info, String::new(), range.start));
            }
            Event::Text(text) if code.is_some() => {
                code.as_mut().expect("code block").1.push_str(&text)
            }
            Event::End(TagEnd::CodeBlock) => {
                let (info, source, start) = code.take().expect("code block");
                blocked.push(start..range.end);
                let span = at(entry, start..range.end);
                let words = info.split_whitespace().collect::<Vec<_>>();
                if words.first() == Some(&"output") {
                    if words.len() != 1 {
                        return Err(Diagnostic::new(
                            "E803",
                            "unknown output-fence attribute",
                            span,
                        ));
                    }
                    let id = last_run.take().ok_or_else(|| {
                        Diagnostic::new("E803", "output fence needs a preceding run example", span)
                    })?;
                    let example: &mut Example = &mut entry.examples[id];
                    example.expected = Some(source);
                } else {
                    last_run = None;
                    if words.first() != Some(&"meowy") {
                        continue;
                    }
                    if words.len() > 2 {
                        return Err(Diagnostic::new(
                            "E803",
                            "unknown documentation example attributes",
                            span,
                        ));
                    }
                    let mode = match words.get(1).copied() {
                        None | Some("check") => ExampleMode::Check,
                        Some("run") => ExampleMode::Run,
                        Some(value) if value.starts_with("reject=") => {
                            let code = &value[7..];
                            let language = code.len() == 4
                                && code.starts_with('E')
                                && code.as_bytes()[1..].iter().all(u8::is_ascii_digit)
                                && matches!(code.as_bytes()[1], b'0'..=b'4' | b'8')
                                && !matches!(code, "E804" | "E805");
                            if !language
                                || !include_str!("../../../docs/reference/diagnostic-codes.md")
                                    .contains(&format!("`{code}`"))
                            {
                                return Err(Diagnostic::new(
                                    "E803",
                                    "reject example requires an assigned language diagnostic, not an unsupported/infrastructure code",
                                    span,
                                ));
                            }
                            ExampleMode::Reject(code.into())
                        }
                        _ => {
                            return Err(Diagnostic::new(
                                "E803",
                                "unknown documentation example attribute",
                                span,
                            ));
                        }
                    };
                    if entry.examples.len() >= 256 {
                        return Err(super::Model::budget(span));
                    }
                    if matches!(mode, ExampleMode::Run) {
                        last_run = Some(entry.examples.len());
                    }
                    entry.examples.push(Example {
                        mode,
                        source,
                        expected: None,
                        span,
                        ran: false,
                    });
                }
            }
            Event::Code(_) | Event::Html(_) | Event::InlineHtml(_) => blocked.push(range),
            Event::Start(Tag::Link { .. } | Tag::Image { .. }) => blocked.push(range),
            _ => {}
        }
    }
    let mut pos = 0;
    while let Some(offset) = entry.text[pos..].find("[[") {
        let start = pos + offset;
        pos = start + 2;
        if blocked.iter().any(|range| range.contains(&start)) {
            continue;
        }
        let slashes = entry.text[..start]
            .bytes()
            .rev()
            .take_while(|byte| *byte == b'\\')
            .count();
        if slashes % 2 != 0 {
            continue;
        }
        let end = entry.text[pos..]
            .find("]]")
            .map(|end| pos + end)
            .ok_or_else(|| {
                Diagnostic::new(
                    "E802",
                    "unclosed documentation link",
                    at(entry, start..entry.text.len()),
                )
            })?;
        let raw = &entry.text[pos..end];
        let (target, label) = raw.split_once('|').unwrap_or((raw, raw));
        let target = target.trim();
        let name = target
            .strip_prefix('<')
            .and_then(|text| text.strip_suffix('>'))
            .unwrap_or(target);
        let valid = if !target.starts_with('<') {
            value_path(target).is_some()
        } else {
            !name.is_empty()
                && name.split('.').all(|part| {
                    let mut bytes = part.bytes();
                    bytes
                        .next()
                        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
                        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                })
        };
        let span = at(entry, start..end + 2);
        if !valid || target.len() > 1024 || label.contains(['\n', '\r', '[', ']', '|']) {
            return Err(Diagnostic::new(
                "E802",
                "documentation links require a name path and a plain label",
                span,
            ));
        }
        if entry.links.len() >= 4096 {
            return Err(super::Model::budget(span));
        }
        entry.links.push(Link {
            target: target.into(),
            label: label.trim().into(),
            range: start..end + 2,
            span,
            resolved: None,
        });
        pos = end + 2;
    }
    Ok(())
}
