use super::{Checker, Node};
use crate::{ast::Span, check::Result, diagnostic::Diagnostic, hir};

pub(crate) const MAX_BODY_FACTS: usize = 262_144;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Fact {
    Bind(usize),
    Read(usize),
    Write(usize),
    Store,
    Alias(usize),
    Emit(hir::EmitId),
    Call(hir::CallId),
    Branch,
    Block(hir::BlockId),
    Leave(hir::BlockId),
    Restart(hir::RestartId),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Body {
    pub(crate) owner: usize,
    pub(crate) facts: Vec<(Fact, Span)>,
}

pub(crate) struct Walk<'a> {
    pub(crate) pending: Vec<Node<'a>>,
    pub(crate) facts: Vec<(Fact, Span)>,
    pub(crate) flow: &'a mut crate::flow::Flow,
    pub(crate) limit: usize,
    pub(crate) span: Span,
}

impl<'a> Walk<'a> {
    pub(crate) fn push(&mut self, nodes: impl IntoIterator<Item = Node<'a>>) -> Result<()> {
        for node in nodes {
            if self.pending.len() + self.facts.len() >= self.limit || !self.flow.spend(1) {
                return Err(Self::budget(self.span));
            }
            self.pending.push(node);
        }
        Ok(())
    }

    pub(crate) fn fact(&mut self, fact: Fact, span: Span) -> Result<()> {
        if self.facts.len() >= self.limit || !self.flow.spend(1) {
            return Err(Self::budget(span));
        }
        self.facts.push((fact, span));
        Ok(())
    }

    pub(crate) fn budget(span: Span) -> Diagnostic {
        Diagnostic::unsupported("proof body fact budget exhausted", span)
    }

    pub(crate) fn path(&mut self, path: &'a [hir::WriteStep]) -> Result<()> {
        for step in path {
            if !self.flow.spend(1) {
                return Err(Self::budget(self.span));
            }
            if let hir::WriteStep::Index(step) = step {
                self.push([Node::Expr(&step.index)])?;
            }
        }
        Ok(())
    }

    pub(crate) fn run(&mut self) -> Result<()> {
        use hir::{ExprKind as E, Stmt as S};
        while let Some(node) = self.pending.pop() {
            match node {
                Node::Expr(expr) => match &expr.kind {
                    E::Local(id) => self.fact(Fact::Read(*id), expr.span)?,
                    E::Borrow(place) | E::ExclusivePath { place, .. } => {
                        self.fact(Fact::Read(place.root), expr.span)?;
                        if let E::ExclusivePath { path, .. } = &expr.kind {
                            self.path(path)?;
                        }
                    }
                    E::TemporaryBorrow { id, value, .. } => {
                        self.fact(Fact::Bind(*id), expr.span)?;
                        self.push([Node::Expr(value)])?;
                    }
                    E::Unary { value, .. }
                    | E::Reborrow { value, .. }
                    | E::Coerce { value }
                    | E::TypeTest { value, .. }
                    | E::Field { value, .. }
                    | E::Deref(value)
                    | E::Primary(value)
                    | E::ListSize(value)
                    | E::StringSize(value) => self.push([Node::Expr(value)])?,
                    E::Binary { left, right, .. } => {
                        self.push([Node::Expr(right), Node::Expr(left)])?;
                    }
                    E::ListIndex { value, index } | E::ElementBorrow { value, index, .. } => {
                        self.push([Node::Expr(index), Node::Expr(value)])?;
                    }
                    E::ListAdd { value, item } => {
                        self.push([Node::Expr(item), Node::Expr(value)])?;
                    }
                    E::Call { site, args, .. } => {
                        self.fact(Fact::Call(*site), expr.span)?;
                        self.push(args.iter().rev().map(Node::Expr))?;
                    }
                    E::List { values, .. }
                    | E::Print { parts: values, .. }
                    | E::Panic { parts: values } => {
                        self.push(values.iter().rev().map(Node::Expr))?;
                    }
                    E::Block(block) => self.fact(Fact::Block(block.id), expr.span)?,
                    E::Null | E::Bool(_) | E::Int(_) | E::Float(_) | E::String(_) | E::Heap => {}
                },
                Node::Stmt(stmt) => match stmt {
                    S::Statement { stmts, .. } => self.push(stmts.iter().rev().map(Node::Stmt))?,
                    S::Bind { id, value } | S::Assign { id, value } => {
                        let fact = if matches!(stmt, S::Bind { .. }) {
                            Fact::Bind(*id)
                        } else {
                            Fact::Write(*id)
                        };
                        self.fact(fact, value.span)?;
                        self.push([Node::Expr(value)])?;
                    }
                    S::SetPath {
                        id,
                        path,
                        value,
                        span,
                    } => {
                        self.fact(Fact::Write(*id), *span)?;
                        self.push([Node::Expr(value)])?;
                        self.path(path)?;
                    }
                    S::Store {
                        target,
                        value,
                        span,
                    } => {
                        self.fact(Fact::Store, *span)?;
                        self.push([Node::Expr(value), Node::Expr(target)])?;
                    }
                    S::Emit { id, value, .. } => {
                        self.fact(Fact::Emit(*id), value.span)?;
                        self.push([Node::Expr(value)])?;
                    }
                    S::If {
                        condition,
                        then,
                        otherwise,
                    } => {
                        self.fact(Fact::Branch, condition.span)?;
                        self.push(otherwise.iter().rev().map(Node::Stmt))?;
                        self.push(then.iter().rev().map(Node::Stmt))?;
                        self.push([Node::Expr(condition)])?;
                    }
                    S::SlotAlias { id, .. } => self.fact(Fact::Alias(*id), self.span)?,
                    S::Leave(target) => self.fact(Fact::Leave(*target), self.span)?,
                    S::Restart { site, .. } => self.fact(Fact::Restart(*site), self.span)?,
                    S::Expr(expr) => self.push([Node::Expr(expr)])?,
                },
            }
        }
        Ok(())
    }
}

impl Checker {
    pub(crate) fn track_required_read(&mut self, expr: &crate::ast::Expr) -> Result<()> {
        use crate::{
            ast::ExprKind,
            check::{InputUse, Value},
        };
        if !matches!(expr.kind, ExprKind::Name(_) | ExprKind::Field { .. }) {
            return Ok(());
        }
        if !self.flow.spend(self.frames.len() + self.scopes.len() + 1) {
            return Err(Walk::budget(expr.span));
        }
        let Some(frame) = self
            .frames
            .iter()
            .rev()
            .find(|frame| frame.owner == self.owner)
        else {
            return Ok(());
        };
        let mut base = expr;
        while let ExprKind::Field { value, .. } | ExprKind::Group(value) = &base.kind {
            if !self.flow.spend(1) {
                return Err(Walk::budget(expr.span));
            }
            base = value;
        }
        let ExprKind::Name(name) = &base.kind else {
            return Ok(());
        };
        let Some(Value::Local { id, .. } | Value::FileModule { id, .. }) = self
            .scopes
            .iter()
            .rev()
            .find_map(|scope| scope.values.get(name))
        else {
            return Ok(());
        };
        if self.body_facts >= MAX_BODY_FACTS
            || !self
                .flow
                .spend(self.body_inputs.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(Walk::budget(expr.span));
        }
        let input = InputUse {
            id: *id,
            span: expr.span,
            control: self.control,
            root: self
                .type_work
                .as_ref()
                .expect("required input root")
                .logical
                .root,
        };
        self.body_inputs.entry(frame.id).or_default().push(input);
        self.body_facts += 1;
        Ok(())
    }

    pub(crate) fn track_body(&mut self, block: &hir::Block, span: Span) -> Result<()> {
        if !self
            .flow
            .spend(self.bodies.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(Walk::budget(span));
        }
        let mut walk = Walk {
            pending: Vec::new(),
            facts: Vec::new(),
            flow: &mut self.flow,
            limit: MAX_BODY_FACTS.saturating_sub(self.body_facts),
            span,
        };
        walk.push(block.stmts.iter().rev().map(Node::Stmt))?;
        walk.run()?;
        let body = Body {
            owner: self.owner,
            facts: walk.facts,
        };
        if let Some(prior) = self.bodies.get(&block.id) {
            if *prior != body {
                return Err(Diagnostic::unsupported(
                    "proof body identity mismatch",
                    span,
                ));
            }
            return Ok(());
        }
        self.body_facts += body.facts.len();
        self.bodies.insert(block.id, body);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
