use super::{Example, Link, MAX_SOURCE, MAX_WORK, Result};
use crate::ast::{Block, Expr, ExprKind, Span, Stmt, StmtKind, StringPart, TypeExpr, TypeKind};
use crate::diagnostic::Diagnostic;
use crate::lexer::{Documentation, TokenKind};
use crate::parser::Parsed;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Module,
    Binding,
    Function,
    Type,
    Parameter,
    Field,
    Emission,
    Forward,
}

#[derive(Debug)]
pub(crate) struct Entry {
    pub(crate) name: String,
    pub(crate) kind: Kind,
    pub(crate) span: Span,
    pub(crate) stage: usize,
    pub(crate) parent: Option<usize>,
    pub(crate) public: bool,
    pub(crate) type_at: Option<usize>,
    pub(crate) signature: String,
    pub(crate) doc: Option<Documentation>,
    pub(crate) text: String,
    pub(crate) map: Vec<usize>,
    pub(crate) links: Vec<Link>,
    pub(crate) examples: Vec<Example>,
    pub(crate) checked: bool,
}

#[derive(Debug)]
pub(crate) struct Model {
    pub(crate) entries: Vec<Entry>,
    pub(crate) starts: BTreeMap<usize, usize>,
    pub(crate) types: BTreeMap<usize, String>,
    pub(crate) work: usize,
    pub(crate) origin: Span,
}

impl Model {
    pub(crate) fn budget(span: Span) -> Diagnostic {
        Diagnostic::unsupported("documentation analysis budget exhausted", span)
    }
    pub(crate) fn spend(&mut self, count: usize) -> Result<()> {
        self.work = self.work.saturating_add(count);
        if self.work > MAX_WORK {
            Err(Self::budget(self.origin))
        } else {
            Ok(())
        }
    }
    pub(crate) fn new(source: &str, parsed: &Parsed) -> Result<Self> {
        Self::at(source, parsed, 0, true)
    }
    pub(crate) fn at(source: &str, parsed: &Parsed, base: usize, standalone: bool) -> Result<Self> {
        if source.len() > MAX_SOURCE {
            return Err(Self::budget(parsed.block.span));
        }
        let source = Source { text: source, base };
        let mut model = Self {
            entries: Vec::new(),
            starts: BTreeMap::new(),
            types: BTreeMap::new(),
            work: 0,
            origin: parsed.block.span,
        };
        model.add(
            "module",
            Kind::Module,
            parsed.block.span,
            usize::MAX,
            None,
            false,
            None,
        )?;
        for stmt in &parsed.block.stmts {
            let public = standalone
                || matches!(
                    stmt.kind,
                    StmtKind::TypeAlias { exported: true, .. }
                        | StmtKind::Emit {
                            label: None,
                            name: Some(_),
                            ..
                        }
                );
            model.walk_stmt(source, parsed, stmt, None, public, 1)?;
        }
        for doc in &parsed.docs {
            let id = if doc.module {
                if parsed
                    .block
                    .stmts
                    .first()
                    .is_some_and(|stmt| doc.open.start >= stmt.span.start)
                {
                    return Err(Diagnostic::new(
                        "E801",
                        "module documentation must precede the first declaration",
                        doc.open,
                    ));
                }
                0
            } else {
                let at = source.skip(doc.close.end)?;
                let id = model.starts.get(&at).copied().ok_or_else(|| {
                    Diagnostic::new(
                        "E801",
                        "documentation must precede a named declaration, parameter or field",
                        doc.open,
                    )
                })?;
                let prior = parsed
                    .marks
                    .iter()
                    .rev()
                    .find(|(_, span)| span.end <= doc.open.start);
                if let Some((kind, span)) = prior
                    && *kind != TokenKind::Newline
                    && !matches!(source.slice(*span), ";" | "{" | "(" | ",")
                {
                    return Err(Diagnostic::new(
                        "E801",
                        "trailing documentation does not attach to the next declaration",
                        doc.open,
                    ));
                }
                id
            };
            if model.entries[id].doc.is_some() {
                return Err(Diagnostic::new(
                    "E801",
                    "duplicate documentation for one declaration",
                    doc.open,
                ));
            }
            let (text, map) = source.normalize(doc.body);
            model.spend(text.len().saturating_mul(2) + 1)?;
            let entry = &mut model.entries[id];
            entry.doc = Some(*doc);
            entry.text = text;
            entry.map = map;
            super::markup::analyze(entry)?;
        }
        Ok(model)
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add(
        &mut self,
        name: &str,
        kind: Kind,
        span: Span,
        stage: usize,
        parent: Option<usize>,
        public: bool,
        type_at: Option<usize>,
    ) -> Result<usize> {
        self.spend(name.len() + 16)?;
        if self.entries.len() >= 16384 {
            return Err(Self::budget(span));
        }
        let id = self.entries.len();
        if kind != Kind::Module && self.starts.insert(span.start, id).is_some() {
            return Err(Self::budget(span));
        }
        self.entries.push(Entry {
            name: name.into(),
            kind,
            span,
            stage,
            parent,
            public,
            type_at,
            signature: String::new(),
            doc: None,
            text: String::new(),
            map: Vec::new(),
            links: Vec::new(),
            examples: Vec::new(),
            checked: false,
        });
        Ok(id)
    }
    pub(crate) fn walk_block(
        &mut self,
        source: Source<'_>,
        parsed: &Parsed,
        block: &Block,
        parent: Option<usize>,
        public: bool,
        depth: usize,
    ) -> Result<()> {
        for stmt in &block.stmts {
            self.walk_stmt(source, parsed, stmt, parent, public, depth + 1)?;
        }
        Ok(())
    }
    pub(crate) fn walk_stmt(
        &mut self,
        source: Source<'_>,
        parsed: &Parsed,
        stmt: &Stmt,
        parent: Option<usize>,
        public: bool,
        depth: usize,
    ) -> Result<()> {
        self.depth(depth, stmt.span)?;
        let stage = stmt.span.start;
        match &stmt.kind {
            StmtKind::Bind {
                name, ty, value, ..
            } => {
                let kind = if matches!(value.kind, ExprKind::Function { .. }) {
                    Kind::Function
                } else {
                    Kind::Binding
                };
                let id = self.add(
                    name,
                    kind,
                    stmt.span,
                    stage,
                    parent,
                    public,
                    ty.as_ref().map(|ty| ty.span.start),
                )?;
                if let Some(ty) = ty {
                    self.walk_type(source, parsed, ty, id, stage, depth + 1)?;
                }
                self.walk_expr(source, parsed, value, Some(id), stage, depth + 1)?;
            }
            StmtKind::TypeAlias { name, ty, .. } | StmtKind::Forward { name, ty } => {
                let kind = if matches!(stmt.kind, StmtKind::TypeAlias { .. }) {
                    Kind::Type
                } else {
                    Kind::Forward
                };
                let id = self.add(name, kind, stmt.span, stage, parent, public, None)?;
                self.walk_type(source, parsed, ty, id, stage, depth + 1)?;
            }
            StmtKind::Emit {
                name, ty, value, ..
            } => {
                let owner = if let Some(name) = name {
                    Some(self.add(
                        name,
                        if matches!(value.kind, ExprKind::Function { .. }) {
                            Kind::Function
                        } else {
                            Kind::Emission
                        },
                        stmt.span,
                        stage,
                        parent,
                        public || parent.is_some_and(|id| self.entries[id].public),
                        ty.as_ref().map(|ty| ty.span.start),
                    )?)
                } else {
                    parent
                };
                if let (Some(ty), Some(id)) = (ty, owner) {
                    self.walk_type(source, parsed, ty, id, stage, depth + 1)?;
                }
                self.walk_expr(source, parsed, value, owner, stage, depth + 1)?;
            }
            StmtKind::Assign { target, value } => {
                self.walk_expr(source, parsed, target, None, stage, depth + 1)?;
                self.walk_expr(source, parsed, value, None, stage, depth + 1)?;
            }
            StmtKind::Expr(value) => {
                self.walk_expr(source, parsed, value, None, stage, depth + 1)?
            }
            StmtKind::Match { arms } => {
                for (condition, body) in arms {
                    if let Some(condition) = condition {
                        self.walk_expr(source, parsed, condition, None, stage, depth + 1)?;
                    }
                    self.walk_stmt(source, parsed, body, parent, false, depth + 1)?;
                }
            }
        }
        Ok(())
    }
    pub(crate) fn walk_type(
        &mut self,
        source: Source<'_>,
        parsed: &Parsed,
        ty: &TypeExpr,
        parent: usize,
        stage: usize,
        depth: usize,
    ) -> Result<()> {
        self.depth(depth, ty.span)?;
        match &ty.kind {
            TypeKind::Record { primary, fields } => {
                if let Some(primary) = primary {
                    self.walk_type(source, parsed, primary, parent, stage, depth + 1)?;
                }
                for (name, ty, _) in fields {
                    let index = parsed
                        .marks
                        .partition_point(|(_, span)| span.end <= ty.span.start);
                    let (_, span) = parsed.marks[..index]
                        .iter()
                        .rev()
                        .find(|(kind, _)| *kind != TokenKind::Newline)
                        .ok_or_else(|| Self::budget(ty.span))?;
                    if source.slice(*span) != name {
                        return Err(Self::budget(*span));
                    }
                    let id = self.add(
                        name,
                        Kind::Field,
                        Span::new(span.start, ty.span.end),
                        stage,
                        Some(parent),
                        self.entries[parent].public,
                        Some(ty.span.start),
                    )?;
                    self.walk_type(source, parsed, ty, id, stage, depth + 1)?;
                }
            }
            TypeKind::Union(types) => {
                for ty in types {
                    self.walk_type(source, parsed, ty, parent, stage, depth + 1)?;
                }
            }
            TypeKind::Function { params, result } => {
                for ty in params {
                    self.walk_type(source, parsed, ty, parent, stage, depth + 1)?;
                }
                self.walk_type(source, parsed, result, parent, stage, depth + 1)?;
            }
            TypeKind::Reference { value, .. } => {
                self.walk_type(source, parsed, value, parent, stage, depth + 1)?
            }
            TypeKind::List { element, size } => {
                self.walk_type(source, parsed, element, parent, stage, depth + 1)?;
                if let Some(size) = size {
                    self.walk_expr(source, parsed, size, None, stage, depth + 1)?;
                }
            }
            TypeKind::Computed(value) => {
                self.walk_expr(source, parsed, value, None, stage, depth + 1)?
            }
            TypeKind::Name(_) | TypeKind::Unsupported(_) => {}
        }
        Ok(())
    }
    pub(crate) fn walk_expr(
        &mut self,
        source: Source<'_>,
        parsed: &Parsed,
        expr: &Expr,
        parent: Option<usize>,
        stage: usize,
        depth: usize,
    ) -> Result<()> {
        self.depth(depth, expr.span)?;
        match &expr.kind {
            ExprKind::Block(block) => {
                self.walk_block(source, parsed, block, parent, false, depth + 1)?
            }
            ExprKind::Function { params, body } => {
                for param in params {
                    let id = self.add(
                        &param.name,
                        Kind::Parameter,
                        param.span,
                        stage,
                        parent,
                        parent.is_some_and(|id| self.entries[id].public),
                        Some(param.ty.span.start),
                    )?;
                    self.walk_type(source, parsed, &param.ty, id, stage, depth + 1)?;
                }
                self.walk_block(source, parsed, body, parent, false, depth + 1)?
            }
            ExprKind::Group(value) => {
                self.walk_expr(source, parsed, value, parent, stage, depth + 1)?
            }
            ExprKind::Unary { value, .. }
            | ExprKind::Field { value, .. }
            | ExprKind::TypeQuery(value) => {
                self.walk_expr(source, parsed, value, None, stage, depth + 1)?
            }
            ExprKind::Binary { left, right, .. } => {
                self.walk_expr(source, parsed, left, None, stage, depth + 1)?;
                self.walk_expr(source, parsed, right, None, stage, depth + 1)?;
            }
            ExprKind::Index { value, index } => {
                self.walk_expr(source, parsed, value, None, stage, depth + 1)?;
                self.walk_expr(source, parsed, index, None, stage, depth + 1)?;
            }
            ExprKind::Call { callee, args } => {
                self.walk_expr(source, parsed, callee, None, stage, depth + 1)?;
                for arg in args {
                    self.walk_expr(source, parsed, arg, None, stage, depth + 1)?;
                }
            }
            ExprKind::Dispatch {
                value,
                callee,
                args,
            } => {
                self.walk_expr(source, parsed, value, None, stage, depth + 1)?;
                self.walk_expr(source, parsed, callee, None, stage, depth + 1)?;
                for arg in args {
                    self.walk_expr(source, parsed, arg, None, stage, depth + 1)?;
                }
            }
            ExprKind::DispatchBlock { value, block } => {
                self.walk_expr(source, parsed, value, None, stage, depth + 1)?;
                self.walk_block(source, parsed, block, None, false, depth + 1)?;
            }
            ExprKind::Ascribe { value, ty, .. } => {
                self.walk_expr(source, parsed, value, parent, stage, depth + 1)?;
                if let Some(id) = parent {
                    self.walk_type(source, parsed, ty, id, stage, depth + 1)?;
                }
            }
            ExprKind::TypeValue(ty) => {
                if let Some(id) = parent {
                    self.walk_type(source, parsed, ty, id, stage, depth + 1)?;
                }
            }
            ExprKind::List(values) => {
                for value in values {
                    self.walk_expr(source, parsed, value, None, stage, depth + 1)?;
                }
            }
            ExprKind::String(parts) => {
                for part in parts {
                    if let StringPart::Value(value) = part {
                        self.walk_expr(source, parsed, value, None, stage, depth + 1)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub(crate) fn depth(&mut self, depth: usize, span: Span) -> Result<()> {
        if depth > 128 {
            return Err(Self::budget(span));
        }
        self.spend(1)
    }
    pub(crate) fn record_type(&mut self, span: Span, ty: &crate::hir::Type) -> Result<()> {
        let text = type_name(ty);
        self.spend(text.len() + 1)?;
        self.types.insert(span.start, text);
        Ok(())
    }
    pub(crate) fn finish(&mut self) -> Result<()> {
        for entry in &mut self.entries {
            if entry.signature.is_empty()
                && let Some(at) = entry.type_at
            {
                entry.signature = self.types.get(&at).cloned().unwrap_or_default();
            }
            if entry.doc.is_some() && !entry.checked {
                return Err(Diagnostic::unsupported(
                    "documentation for an unanalyzed declaration",
                    entry.span,
                ));
            }
        }
        Ok(())
    }
    pub(crate) fn require_public(&self) -> Result<()> {
        if let Some(entry) = self.entries.iter().find(|entry| {
            entry.public && entry.kind != Kind::Parameter && entry.text.trim().is_empty()
        }) {
            return Err(Diagnostic::new(
                "E803",
                format!("public declaration `{}` needs documentation", entry.name),
                entry.span,
            ));
        }
        Ok(())
    }
    pub(crate) fn member(&self, parent: usize, name: &str) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.parent == Some(parent) && entry.name == name)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Source<'a> {
    pub(crate) text: &'a str,
    pub(crate) base: usize,
}

impl Source<'_> {
    pub(crate) fn slice(&self, span: Span) -> &str {
        &self.text[span.start - self.base..span.end - self.base]
    }

    pub(crate) fn skip(&self, pos: usize) -> Result<usize> {
        skip(self.text, pos - self.base)
            .map(|pos| pos + self.base)
            .map_err(|mut error| {
                error.span.start += self.base;
                error.span.end += self.base;
                error
            })
    }

    pub(crate) fn normalize(&self, span: Span) -> (String, Vec<usize>) {
        let span = Span::new(span.start - self.base, span.end - self.base);
        let (text, mut map) = normalize(self.text, span);
        for pos in &mut map {
            *pos += self.base;
        }
        (text, map)
    }
}

pub(crate) fn type_name(ty: &crate::hir::Type) -> String {
    use crate::hir::Type;
    match ty {
        Type::Null => "null".into(),
        Type::Never => "never".into(),
        Type::Bool => "boolean".into(),
        Type::String => "string".into(),
        Type::Int { bits, signed } => format!("{}{bits}", if *signed { "int" } else { "uint" }),
        Type::Float { bits } => format!("float{bits}"),
        Type::Foundation(ty) => ty.name().into(),
        Type::Reference(ty) => format!("&{}", type_name(ty)),
        Type::Exclusive(ty) => format!("&!{}", type_name(ty)),
        Type::List { element, capacity } => format!("{}[{capacity}]", type_name(element)),
        Type::Union(types) => types
            .iter()
            .map(|ty| format!("<{}>", type_name(ty)))
            .collect::<String>(),
        Type::Record { primary, fields } => {
            let mut parts = Vec::new();
            if **primary != Type::Null {
                parts.push(format!("-><{}>", type_name(primary)));
            }
            parts.extend(fields.iter().map(|field| {
                format!(
                    "{}<{}>{}",
                    field.name,
                    type_name(&field.ty),
                    if field.mutable { ":=" } else { "" }
                )
            }));
            format!("{{{}}}", parts.join(";"))
        }
    }
}

pub(crate) fn normalize(source: &str, span: Span) -> (String, Vec<usize>) {
    let body = &source[span.start..span.end];
    let indent = body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.bytes().take_while(|byte| *byte == b' ').count())
        .min()
        .unwrap_or(0);
    let mut bytes = Vec::new();
    let mut map = Vec::new();
    let mut offset = span.start;
    for line in body.split_inclusive('\n') {
        let trim = line
            .bytes()
            .take_while(|byte| *byte == b' ')
            .count()
            .min(indent);
        for (index, byte) in line.bytes().enumerate().skip(trim) {
            if byte == b'\r' && line.as_bytes().get(index + 1) == Some(&b'\n') {
                continue;
            }
            bytes.push(byte);
            map.push(offset + index);
        }
        offset += line.len();
    }
    map.push(span.end);
    (
        String::from_utf8(bytes).expect("UTF-8-preserving normalization"),
        map,
    )
}

pub(crate) fn skip(source: &str, mut pos: usize) -> Result<usize> {
    while pos < source.len() {
        if source.as_bytes()[pos].is_ascii_whitespace() {
            pos += 1;
        } else if source.as_bytes()[pos] == b'#' {
            let mut errors = Vec::new();
            let (end, kind) = crate::lexer::scan_comment(source, pos, &mut errors);
            if let Some(error) = errors.into_iter().next() {
                return Err(error);
            }
            if matches!(kind, TokenKind::Doc { .. }) {
                return Err(Diagnostic::new(
                    "E801",
                    "multiple documentation blocks before a declaration",
                    Span::new(pos, end),
                ));
            }
            pos = end;
        } else {
            break;
        }
    }
    Ok(pos)
}
