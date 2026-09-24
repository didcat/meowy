use super::{Checker, Node};
use crate::{ast::Span, check::Result, diagnostic::Diagnostic, hir};

pub(crate) const MAX_BODY_FACTS: usize = 262_144;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    Data,
    Condition,
    Then,
    Else,
    Index,
    Address,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Link {
    pub(crate) parent: usize,
    pub(crate) role: Role,
}

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
    And,
    Or,
    Block(hir::BlockId),
    Leave(hir::BlockId),
    Restart(hir::RestartId),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Body {
    pub(crate) owner: usize,
    pub(crate) facts: Vec<(Fact, Span)>,
    pub(crate) links: Vec<Option<Link>>,
    pub(crate) storage: Vec<Option<hir::LocalId>>,
}

pub(crate) struct Walk<'a> {
    pub(crate) pending: Vec<(Node<'a>, Option<Link>)>,
    pub(crate) facts: Vec<(Fact, Span)>,
    pub(crate) links: Vec<Option<Link>>,
    pub(crate) storage: Vec<Option<hir::LocalId>>,
    pub(crate) link: Option<Link>,
    pub(crate) aliases: &'a std::collections::BTreeMap<hir::LocalId, crate::borrow::Alias>,
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
            self.pending.push((node, self.link));
        }
        Ok(())
    }

    pub(crate) fn fact(&mut self, fact: Fact, span: Span) -> Result<()> {
        if self.facts.len() >= self.limit || !self.flow.spend(1) {
            return Err(Self::budget(span));
        }
        let storage = match &fact {
            Fact::Bind(id) | Fact::Read(id) | Fact::Write(id) | Fact::Alias(id) => {
                if !self
                    .flow
                    .spend(self.aliases.len().checked_ilog2().unwrap_or(0) as usize + 1)
                {
                    return Err(Self::budget(span));
                }
                Some(self.aliases.get(id).map_or(*id, |alias| alias.root))
            }
            _ => None,
        };
        let parent = self.facts.len();
        self.facts.push((fact, span));
        self.links.push(self.link);
        self.storage.push(storage);
        self.link = Some(Link {
            parent,
            role: Role::Data,
        });
        Ok(())
    }

    pub(crate) fn role(&mut self, role: Role) {
        if let Some(link) = &mut self.link {
            link.role = role;
        }
    }

    pub(crate) fn budget(span: Span) -> Diagnostic {
        Diagnostic::unsupported("proof body fact budget exhausted", span)
    }

    pub(crate) fn path(&mut self, path: &'a [hir::WriteStep]) -> Result<()> {
        let link = self.link;
        self.role(Role::Index);
        for step in path {
            if !self.flow.spend(1) {
                return Err(Self::budget(self.span));
            }
            if let hir::WriteStep::Index(step) = step {
                self.push([Node::Expr(&step.index)])?;
            }
        }
        self.link = link;
        Ok(())
    }

    pub(crate) fn run(&mut self) -> Result<()> {
        use hir::{ExprKind as E, Stmt as S};
        while let Some((node, link)) = self.pending.pop() {
            self.link = link;
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
                    E::Binary { op, left, right } if matches!(op.as_str(), "&&" | "||") => {
                        let and = op == "&&";
                        self.fact(if and { Fact::And } else { Fact::Or }, expr.span)?;
                        self.role(if and { Role::Then } else { Role::Else });
                        self.push([Node::Expr(right)])?;
                        self.role(Role::Condition);
                        self.push([Node::Expr(left)])?;
                    }
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
                        self.push([Node::Expr(value)])?;
                        self.role(Role::Address);
                        self.push([Node::Expr(target)])?;
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
                        self.role(Role::Else);
                        self.push(otherwise.iter().rev().map(Node::Stmt))?;
                        self.role(Role::Then);
                        self.push(then.iter().rev().map(Node::Stmt))?;
                        self.role(Role::Condition);
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
        let site = self.checked_site(expr.span)?;
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
            || !self.flow.spend(
                self.body_inputs.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                    + 2,
            )
        {
            return Err(Walk::budget(expr.span));
        }
        let input = InputUse {
            site,
            id: *id,
            storage: self.proofs.aliases.get(id).map_or(*id, |alias| alias.root),
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
            links: Vec::new(),
            storage: Vec::new(),
            link: None,
            aliases: &self.proofs.aliases,
            flow: &mut self.flow,
            limit: MAX_BODY_FACTS.saturating_sub(self.body_facts),
            span,
        };
        walk.push(block.stmts.iter().rev().map(Node::Stmt))?;
        walk.run()?;
        let body = Body {
            owner: self.owner,
            facts: walk.facts,
            links: walk.links,
            storage: walk.storage,
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

#[cfg(test)]
mod relations;

#[cfg(test)]
mod logic;
