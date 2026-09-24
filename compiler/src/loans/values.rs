use super::access::Kind;
use super::{
    BTreeMap, Bundle, CallId, Diagnostic, Expr, ExprKind, FALSE, Graph, LocalId, MAX_ORIGINS,
    MAX_VALUES, Node, Origin, Path, Result, Span, Step, Type,
};

#[derive(Clone, Default)]
pub(crate) struct Value {
    pub(crate) origins: Vec<Origin>,
    pub(crate) bounds: Vec<Origin>,
    pub(crate) allocator: bool,
}

impl Value {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Origin> {
        self.origins.iter().chain(&self.bounds)
    }

    pub(crate) fn physical(&self) -> impl Iterator<Item = &Origin> {
        self.origins
            .iter()
            .chain(self.bounds.iter().filter(|_| !self.allocator))
    }

    pub(crate) fn len(&self) -> usize {
        self.origins.len() + self.bounds.len()
    }
}

impl From<Vec<Origin>> for Value {
    fn from(origins: Vec<Origin>) -> Self {
        Self {
            origins,
            bounds: Vec::new(),
            allocator: false,
        }
    }
}

impl From<&crate::borrow_value::State> for Value {
    fn from(state: &crate::borrow_value::State) -> Self {
        Self {
            origins: state.origins.clone(),
            bounds: state.bounds.clone(),
            allocator: false,
        }
    }
}

impl<'a> Graph<'a> {
    pub(crate) fn value(&mut self, value: impl Into<Value>) -> Result<usize> {
        let value = value.into();
        self.charge(value.len() + 1)?;
        let weight = value.iter().map(Origin::weight).sum::<usize>();
        if self.values.len() == MAX_VALUES || self.origins + weight > MAX_ORIGINS {
            return Err(Self::budget());
        }
        self.origins += weight;
        let id = self.values.len();
        self.values.push(value);
        Ok(id)
    }

    pub(crate) fn bundle(&mut self, value: impl Into<Value>, ty: &Type) -> Result<Bundle> {
        let value = value.into();
        let mut groups = BTreeMap::<Path, Value>::new();
        for (origins, bound) in [(value.origins, false), (value.bounds, true)] {
            for origin in origins {
                let value = groups.entry(origin.component.clone()).or_default();
                if bound {
                    value.bounds.push(origin);
                } else {
                    value.origins.push(origin);
                }
            }
        }
        for path in crate::borrow_contract::allocator_paths(ty, self.guards, Span::default())? {
            self.charge(path.len() + 1)?;
            groups.entry(path).or_default();
        }
        groups
            .into_iter()
            .map(|(path, mut value)| {
                self.charge(path.len() + 1)?;
                value.allocator = matches!(
                    crate::borrow_contract::component_type(ty, &path),
                    Some(Type::Foundation(crate::hir::FoundationType::Allocator))
                );
                Ok((path, self.value(value)?))
            })
            .collect()
    }

    pub(crate) fn select(value: Bundle, path: &[Step]) -> Bundle {
        value
            .into_iter()
            .filter_map(|(key, id)| key.strip_prefix(path).map(|path| (path.to_vec(), id)))
            .collect()
    }

    pub(crate) fn version(&mut self, source: &Bundle) -> Result<Bundle> {
        let mut value = Bundle::new();
        for (path, id) in source {
            let weight = self.values[*id].iter().map(Origin::weight).sum::<usize>();
            self.charge(weight + path.len() + 1)?;
            value.insert(path.clone(), self.value(self.values[*id].clone())?);
        }
        Ok(value)
    }

    pub(crate) fn field_version(
        &mut self,
        id: LocalId,
        fields: &[usize],
        value: &Bundle,
    ) -> Result<(Bundle, Node)> {
        if fields.is_empty() || !self.merging {
            return Err(Self::budget());
        }
        let prefix = fields
            .iter()
            .map(|index| Step::Slot(index + 1))
            .collect::<Vec<_>>();
        let mut current = self.local(id)?;
        self.charge((current.len() + value.len() + 1).saturating_mul(prefix.len() + 1))?;
        current.retain(|path, _| !path.starts_with(&prefix));
        let target = self.version(value)?;
        let node = self.copied(value, &target)?;
        for (path, value) in target {
            let path = prefix.iter().chain(&path).copied().collect();
            current.insert(path, value);
        }
        Ok((current, node))
    }

    pub(crate) fn copy(&mut self, source: Bundle) -> Result<Bundle> {
        let value = self.version(&source)?;
        let node = self.copied(&source, &value)?;
        self.append(node)?;
        Ok(value)
    }

    pub(crate) fn local(&mut self, id: LocalId) -> Result<Bundle> {
        if let Some(value) = self.locals.get(&id) {
            return Ok(value.clone());
        }
        let origins = self
            .facts
            .locals
            .get(&id)
            .map(Value::from)
            .unwrap_or_default();
        let value = self.bundle(origins, &self.program.locals[id])?;
        self.locals.insert(id, value.clone());
        Ok(value)
    }

    pub(crate) fn expression(&mut self, expr: &Expr) -> Result<Bundle> {
        self.project(expr, &[])
    }

    pub(crate) fn reference_value(&mut self, expr: &Expr) -> Result<Bundle> {
        self.project_mode(expr, &[], false)
    }

    pub(crate) fn call(
        &mut self,
        site: CallId,
        args: &[Expr],
        span: Span,
        ty: &Type,
    ) -> Result<Bundle> {
        self.charge(args.len() + 1)?;
        let returning = crate::borrow_contract::reference_call(ty, args.iter().map(|arg| &arg.ty));
        let scalar =
            returning || crate::borrow_contract::scalar_call(ty, args.iter().map(|arg| &arg.ty));
        let mut inputs = Vec::new();
        let mut uses = Vec::new();
        let mut accesses = Vec::new();
        for arg in args {
            let value = self.expression(arg)?;
            if scalar
                && crate::borrow_contract::scalar_reference(&arg.ty)
                && !self.current.is_empty()
            {
                let kind = if matches!(arg.ty, Type::Exclusive(_)) {
                    Kind::Write
                } else {
                    Kind::Read
                };
                let access = self.pointee_access(&value, &[], kind, arg.span)?;
                accesses.push(access);
            }
            uses.extend(value.values().copied());
            if returning {
                inputs.push(value);
            }
            if self.current.is_empty() {
                return Ok(Bundle::new());
            }
        }
        for access in accesses {
            self.append(Node {
                access: Some(access),
                ..Node::default()
            })?;
        }
        let state = self.facts.calls.get(&site);
        let origins = state.map(Value::from).unwrap_or_default();
        let proof = state
            .map(|state| self.guards.and(state.present, state.proof))
            .unwrap_or(FALSE);
        let value = self.bundle(origins, ty)?;
        let mut node = Node {
            uses,
            barrier: (!scalar).then_some(span),
            defs: if returning {
                Vec::new()
            } else {
                value.values().copied().collect()
            },
            ..Node::default()
        };
        if !self.proofs.carried.is_empty() && args.iter().any(|arg| arg.ty.has_exclusive()) {
            node.emissions.push(super::emission_init::Event::Forget);
        }
        if !returning {
            for id in value.values() {
                self.opaque_value(&mut node, *id)?;
            }
        }
        let node = self.append(node)?;
        if state.is_none() {
            self.missing_calls.push((node, span));
        }
        self.assume(proof)?;
        if returning {
            self.returned(site, &inputs, &value, ty, span)?;
        }
        Ok(value)
    }

    pub(crate) fn inspect(&mut self, expr: &Expr) -> Result<()> {
        self.inspect_path(expr, &[])
    }

    pub(crate) fn inspect_path(&mut self, expr: &Expr, path: &[Step]) -> Result<()> {
        self.charge(path.len() + 1)?;
        match &expr.kind {
            ExprKind::Local(id) => {
                if self.has_tag(&expr.ty, path)? {
                    self.read_local(*id, path, Kind::Tag, expr.span, false)?;
                } else if expr.ty.has_exclusive() {
                    self.read_local(*id, path, Kind::Read, expr.span, false)?;
                }
            }
            ExprKind::Field { value, index } => {
                let path = std::iter::once(Step::Slot(index + 1))
                    .chain(path.iter().copied())
                    .collect::<Vec<_>>();
                self.inspect_path(value, &path)?;
            }
            ExprKind::Primary(value) => {
                let path = std::iter::once(Step::Slot(0))
                    .chain(path.iter().copied())
                    .collect::<Vec<_>>();
                self.inspect_path(value, &path)?;
            }
            ExprKind::Coerce { value } => {
                let path = self.inspection_path(&value.ty, &expr.ty, path)?;
                self.inspect_path(value, &path)?;
            }
            ExprKind::Block(block) => {
                self.block(block)?;
            }
            ExprKind::Deref(value) => {
                let value = self.reference_value(value)?;
                if self.current.is_empty() {
                    return Ok(());
                }
                let uses = self.direct(&value)?;
                let access = if self.has_tag(&expr.ty, path)? {
                    Some(self.pointee_access(&value, path, Kind::Tag, expr.span)?)
                } else {
                    None
                };
                self.append(Node {
                    uses,
                    access,
                    ..Node::default()
                })?;
            }
            _ => {
                self.expression(expr)?;
            }
        }
        Ok(())
    }

    pub(crate) fn retag(
        &mut self,
        source: Bundle,
        sources: &[Type],
        targets: &[Type],
        span: Span,
    ) -> Result<Bundle> {
        let unsupported = || Diagnostic::unsupported("unknown borrowed union projection", span);
        let mut result = Bundle::new();
        for (mut component, id) in source {
            self.charge(component.len() + targets.len() + 1)?;
            let Some(Step::Variant(index)) = component.first() else {
                return Err(unsupported());
            };
            let member = sources.get(*index).ok_or_else(unsupported)?;
            if let Some(index) = targets.iter().position(|ty| ty == member) {
                component[0] = Step::Variant(index);
                result.insert(component, id);
            }
        }
        Ok(result)
    }

    pub(crate) fn convert(&mut self, value: &Expr, to: &Type, path: &[Step]) -> Result<Bundle> {
        self.convert_mode(value, to, path, true)
    }

    pub(crate) fn convert_mode(
        &mut self,
        value: &Expr,
        to: &Type,
        path: &[Step],
        take: bool,
    ) -> Result<Bundle> {
        let from = &value.ty;
        self.charge(path.len() + from.members().len() + to.members().len() + 1)?;
        if from == to {
            return self.project_mode(value, path, take);
        }
        if *from == Type::Never {
            self.inspect(value)?;
            return Ok(Bundle::new());
        }
        let unsupported =
            || Diagnostic::unsupported("unknown borrowed union projection", value.span);
        if let Type::Union(sources) = from {
            if let Type::Union(targets) = to {
                if let Some((Step::Variant(index), tail)) = path.split_first() {
                    let member = targets.get(*index).ok_or_else(unsupported)?;
                    let Some(index) = sources.iter().position(|ty| ty == member) else {
                        self.inspect(value)?;
                        return Ok(Bundle::new());
                    };
                    let path = std::iter::once(Step::Variant(index))
                        .chain(tail.iter().copied())
                        .collect::<Vec<_>>();
                    return self.project_mode(value, &path, take);
                }
                if !path.is_empty() {
                    return Err(unsupported());
                }
                let source = self.project_mode(value, &[], take)?;
                return self.retag(source, sources, targets, value.span);
            }
            let index = sources
                .iter()
                .position(|ty| ty == to)
                .ok_or_else(unsupported)?;
            let path = std::iter::once(Step::Variant(index))
                .chain(path.iter().copied())
                .collect::<Vec<_>>();
            return self.project_mode(value, &path, take);
        }
        if let Type::Union(targets) = to {
            let member = targets
                .iter()
                .position(|ty| ty == from)
                .ok_or_else(unsupported)?;
            if let Some((Step::Variant(index), tail)) = path.split_first() {
                if *index == member {
                    return self.project_mode(value, tail, take);
                }
                self.inspect(value)?;
                return Ok(Bundle::new());
            }
            if !path.is_empty() {
                return Err(unsupported());
            }
            let source = self.project_mode(value, &[], take)?;
            let mut result = Bundle::new();
            for (mut component, id) in source {
                self.charge(component.len() + 1)?;
                component.insert(0, Step::Variant(member));
                result.insert(component, id);
            }
            return Ok(result);
        }
        Err(unsupported())
    }

    pub(crate) fn project(&mut self, expr: &Expr, path: &[Step]) -> Result<Bundle> {
        self.project_mode(expr, path, true)
    }

    pub(crate) fn project_mode(
        &mut self,
        expr: &Expr,
        path: &[Step],
        take: bool,
    ) -> Result<Bundle> {
        self.charge(path.len() + 1)?;
        let value = match &expr.kind {
            ExprKind::TemporaryBorrow {
                id,
                statement,
                value,
            } => {
                self.charge(self.active.len() + 1)?;
                if self.statement_scope(*statement).is_none()
                    || self.proofs.temporaries.get(id) != Some(statement)
                {
                    return Err(Diagnostic::unsupported(
                        "missing temporary statement ownership proof",
                        expr.span,
                    ));
                }
                self.register_store(*id, expr.span)?;
                let value = self.expression(value)?;
                if self.current.is_empty() {
                    return Ok(Bundle::new());
                }
                let uses = self.direct(&value)?;
                let result = self.referenced(
                    super::Source::Temporary {
                        id: *id,
                        statement: *statement,
                        fields: Vec::new(),
                    },
                    value,
                    uses,
                    expr.span,
                    crate::hir::ReferenceMode::Shared,
                )?;
                Self::select(result, path)
            }
            ExprKind::ExclusivePath { .. } => Self::select(self.exclusive_path(expr)?, path),
            ExprKind::Borrow(place) => Self::select(
                self.borrowed(
                    place,
                    expr.span,
                    expr.ty.reference_mode().ok_or_else(Self::budget)?,
                )?,
                path,
            ),
            ExprKind::Reborrow { .. } | ExprKind::ElementBorrow { .. } => {
                self.derived(expr, path)?
            }
            ExprKind::Local(id) if expr.ty.has_borrowed() => {
                self.read_local(*id, path, Kind::Read, expr.span, take)?;
                let local = Self::select(self.local(*id)?, path);
                self.copy(local)?
            }
            ExprKind::Coerce { value } => self.convert_mode(value, &expr.ty, path, take)?,
            ExprKind::Block(block) => Self::select(self.block(block)?, path),
            ExprKind::Field { value, index } => {
                let prefix: Vec<_> = std::iter::once(Step::Slot(index + 1))
                    .chain(path.iter().copied())
                    .collect();
                self.project_mode(value, &prefix, take)?
            }
            ExprKind::Primary(value) => {
                let prefix: Vec<_> = std::iter::once(Step::Slot(0))
                    .chain(path.iter().copied())
                    .collect();
                self.project_mode(value, &prefix, take)?
            }
            ExprKind::Deref(value) => self.dereferenced(value, path)?,
            ExprKind::Unary { value, .. }
            | ExprKind::StringSize(value)
            | ExprKind::ListSize(value) => {
                let value = self.expression(value)?;
                let uses = self.direct(&value)?;
                self.append(Node {
                    uses,
                    ..Node::default()
                })?;
                Bundle::new()
            }
            ExprKind::List { values, .. } => {
                for value in values {
                    let value = self.expression(value)?;
                    if self.current.is_empty() {
                        return Ok(Bundle::new());
                    }
                    let uses = self.direct(&value)?;
                    self.append(Node {
                        uses,
                        ..Node::default()
                    })?;
                }
                Bundle::new()
            }
            ExprKind::ListIndex { value, index } | ExprKind::ListAdd { value, item: index } => {
                let left = self.expression(value)?;
                if self.current.is_empty() {
                    return Ok(Bundle::new());
                }
                let right = self.expression(index)?;
                let mut uses = self.direct(&left)?;
                uses.extend(self.direct(&right)?);
                self.append(Node {
                    uses,
                    ..Node::default()
                })?;
                Bundle::new()
            }
            ExprKind::TypeTest { value, .. } => {
                self.inspect(value)?;
                if let Some(proof) = self
                    .facts
                    .inspections
                    .get(&(expr.span.start, expr.span.end))
                {
                    self.assume(*proof)?;
                }
                Bundle::new()
            }
            ExprKind::Binary {
                op, left, right, ..
            } if ["&&", "||"].contains(&op.as_str()) => {
                self.expression(left)?;
                if self.current.is_empty() {
                    return Ok(Bundle::new());
                }
                let guard = self.condition(left)?;
                let test = self.emission_bool(left)?;
                let guard = if op == "&&" {
                    guard
                } else {
                    self.guards.not(guard)
                };
                let test = if op == "&&" {
                    test
                } else {
                    test.map(super::emission_value::Value::negated)
                };
                self.conditional(
                    guard,
                    test,
                    |graph| graph.expression(right).map(|_| ()),
                    |_| Ok(()),
                )?;
                Bundle::new()
            }
            ExprKind::Binary { left, right, .. } => {
                let left = self.expression(left)?;
                let right = self.expression(right)?;
                let mut uses = self.direct(&left)?;
                uses.extend(self.direct(&right)?);
                self.append(Node {
                    uses,
                    ..Node::default()
                })?;
                Bundle::new()
            }
            ExprKind::Call { site, args, .. } => {
                Self::select(self.call(*site, args, expr.span, &expr.ty)?, path)
            }
            ExprKind::Print { parts, .. } | ExprKind::Panic { parts } => {
                for part in parts {
                    let value = self.expression(part)?;
                    let uses = self.direct(&value)?;
                    self.append(Node {
                        uses,
                        ..Node::default()
                    })?;
                }
                if matches!(expr.kind, ExprKind::Panic { .. }) {
                    self.end_scopes(None)?;
                    self.current.clear();
                }
                Bundle::new()
            }
            ExprKind::Local(id) => {
                self.read_local(*id, path, Kind::Read, expr.span, take)?;
                Bundle::new()
            }
            ExprKind::Null
            | ExprKind::Bool(_)
            | ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Heap => Bundle::new(),
        };
        let value = if self.current.is_empty()
            || !value.is_empty()
            || expr.ty != Type::Foundation(crate::hir::FoundationType::Allocator)
            || !path.is_empty()
        {
            value
        } else {
            let value = self.bundle(Value::default(), &expr.ty)?;
            self.append(Node {
                defs: value.values().copied().collect(),
                ..Node::default()
            })?;
            value
        };
        if expr.ty == Type::Never {
            self.current.clear();
        }
        Ok(value)
    }
}
