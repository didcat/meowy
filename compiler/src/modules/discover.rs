use crate::ast::{Block, Expr, ExprKind, Span, Stmt, StmtKind, StringPart, TypeExpr, TypeKind};
use crate::diagnostic::Diagnostic;

pub(crate) const MAX_WORK: usize = 262_144;

pub(crate) enum Node<'a> {
    Block(&'a Block),
    Stmt(&'a Stmt),
    Expr(&'a Expr),
    Type(&'a TypeExpr),
}

pub(crate) struct Scan<'a> {
    pub(crate) pending: Vec<Node<'a>>,
    pub(crate) work: usize,
    pub(crate) span: Span,
}

impl<'a> Scan<'a> {
    pub(crate) fn push(&mut self, node: Node<'a>) -> Result<(), Diagnostic> {
        if self.work >= MAX_WORK {
            return Err(Diagnostic::unsupported(
                "import discovery work exhausted",
                self.span,
            ));
        }
        self.work += 1;
        self.pending.push(node);
        Ok(())
    }

    pub(crate) fn children(&mut self, node: Node<'a>) -> Result<(), Diagnostic> {
        match node {
            Node::Block(block) => {
                for stmt in &block.stmts {
                    self.push(Node::Stmt(stmt))?;
                }
            }
            Node::Stmt(stmt) => match &stmt.kind {
                StmtKind::Bind { ty, value, .. } | StmtKind::Emit { ty, value, .. } => {
                    if let Some(ty) = ty {
                        self.push(Node::Type(ty))?;
                    }
                    self.push(Node::Expr(value))?;
                }
                StmtKind::TypeAlias { ty, .. } | StmtKind::Forward { ty, .. } => {
                    self.push(Node::Type(ty))?
                }
                StmtKind::Assign { target, value } => {
                    self.push(Node::Expr(target))?;
                    self.push(Node::Expr(value))?;
                }
                StmtKind::Match { arms } => {
                    for (condition, body) in arms {
                        if let Some(condition) = condition {
                            self.push(Node::Expr(condition))?;
                        }
                        self.push(Node::Stmt(body))?;
                    }
                }
                StmtKind::Expr(value) => self.push(Node::Expr(value))?,
            },
            Node::Expr(expr) => match &expr.kind {
                ExprKind::Group(value)
                | ExprKind::Unary { value, .. }
                | ExprKind::Field { value, .. }
                | ExprKind::TypeQuery(value) => self.push(Node::Expr(value))?,
                ExprKind::Binary { left, right, .. } => {
                    self.push(Node::Expr(left))?;
                    self.push(Node::Expr(right))?;
                }
                ExprKind::Index { value, index } => {
                    self.push(Node::Expr(value))?;
                    self.push(Node::Expr(index))?;
                }
                ExprKind::Call { callee, args } | ExprKind::Dispatch { callee, args, .. } => {
                    self.push(Node::Expr(callee))?;
                    for arg in args {
                        self.push(Node::Expr(arg))?;
                    }
                    if let ExprKind::Dispatch { value, .. } = &expr.kind {
                        self.push(Node::Expr(value))?;
                    }
                }
                ExprKind::Block(block) => self.push(Node::Block(block))?,
                ExprKind::Function { params, body } => {
                    for param in params {
                        self.push(Node::Type(&param.ty))?;
                    }
                    self.push(Node::Block(body))?;
                }
                ExprKind::DispatchBlock { value, block } => {
                    self.push(Node::Expr(value))?;
                    self.push(Node::Block(block))?;
                }
                ExprKind::Ascribe { value, ty, .. } => {
                    self.push(Node::Expr(value))?;
                    self.push(Node::Type(ty))?;
                }
                ExprKind::Specialize { value, types } => {
                    self.push(Node::Expr(value))?;
                    for ty in types {
                        self.push(Node::Type(ty))?;
                    }
                }
                ExprKind::TypeValue(ty) => self.push(Node::Type(ty))?,
                ExprKind::List(values) => {
                    for value in values {
                        self.push(Node::Expr(value))?;
                    }
                }
                ExprKind::String(parts) => {
                    for part in parts {
                        if let StringPart::Value(value) = part {
                            self.push(Node::Expr(value))?;
                        }
                    }
                }
                ExprKind::Import(_)
                | ExprKind::Name(_)
                | ExprKind::Int(_)
                | ExprKind::Float(_)
                | ExprKind::Label(_)
                | ExprKind::Unsupported(_) => {}
            },
            Node::Type(ty) => match &ty.kind {
                TypeKind::Union(types) => {
                    for ty in types {
                        self.push(Node::Type(ty))?;
                    }
                }
                TypeKind::Function { params, result } => {
                    for ty in params {
                        self.push(Node::Type(ty))?;
                    }
                    self.push(Node::Type(result))?;
                }
                TypeKind::Record { primary, fields } => {
                    if let Some(ty) = primary {
                        self.push(Node::Type(ty))?;
                    }
                    for (_, ty, _) in fields {
                        self.push(Node::Type(ty))?;
                    }
                }
                TypeKind::List { element, size } => {
                    self.push(Node::Type(element))?;
                    if let Some(size) = size {
                        self.push(Node::Expr(size))?;
                    }
                }
                TypeKind::Reference { value, .. } => self.push(Node::Type(value))?,
                TypeKind::Computed(value) => self.push(Node::Expr(value))?,
                TypeKind::Name(_) | TypeKind::Unsupported(_) => {}
            },
        }
        Ok(())
    }
}

pub(crate) fn imports(block: &Block) -> Result<Vec<(String, Span)>, Diagnostic> {
    let mut scan = Scan {
        pending: Vec::new(),
        work: 0,
        span: block.span,
    };
    scan.push(Node::Block(block))?;
    let mut imports = Vec::new();
    while let Some(node) = scan.pending.pop() {
        if let Node::Expr(Expr {
            kind: ExprKind::Import(name),
            span,
        }) = &node
            && (name.starts_with("./") || name.starts_with("../"))
        {
            if imports.len() >= super::MAX_EDGES {
                return Err(Diagnostic::unsupported(
                    "file-module import budget exhausted",
                    *span,
                ));
            }
            imports.push((name.clone(), *span));
        }
        scan.children(node)?;
    }
    imports.sort_by_key(|(_, span)| span.start);
    if imports
        .windows(2)
        .any(|pair| pair[0].1.start == pair[1].1.start)
    {
        return Err(Diagnostic::unsupported(
            "duplicate import source site",
            block.span,
        ));
    }
    Ok(imports)
}
