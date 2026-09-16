use crate::ast::{self, ExprKind, Span};
use crate::check::{Checker, Constant, Frame, Result, Value};
use crate::diagnostic::Diagnostic;
use crate::flow::FALSE;
use crate::hir::{self, Type};

pub(crate) const MAX_CAPACITY: usize = 65_536;
pub(crate) const MAX_BYTES: usize = 1_048_576;
pub(crate) const MAX_WRITE_PATH: usize = 256;

#[derive(Clone)]
pub(crate) struct Fact {
    pub(crate) ty: Type,
    pub(crate) length: usize,
}

impl Checker {
    pub(crate) fn extent_form(expr: &ast::Expr) -> bool {
        match &expr.kind {
            ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::String(_) | ExprKind::Name(_) => true,
            ExprKind::Group(value) | ExprKind::Unary { value, .. } => Self::extent_form(value),
            ExprKind::Binary { left, right, .. } => {
                Self::extent_form(left) && Self::extent_form(right)
            }
            _ => false,
        }
    }

    pub(crate) fn list_extent(&mut self, expr: &ast::Expr) -> Result<usize> {
        if !self.proven_inputs() {
            if !Self::extent_form(expr) {
                return Err(Diagnostic::unsupported(
                    "required evaluation of list extent expressions",
                    expr.span,
                ));
            }
            return self.required_root(expr.span, |checker| {
                checker.type_work.as_mut().unwrap().extent_only = true;
                checker.extent_value(expr)
            });
        }
        if self.integer_blocks(expr)? {
            let Value::Static {
                value: Constant::Int(value),
                ..
            } = self.integer_arithmetic(expr, None)?
            else {
                unreachable!()
            };
            return usize::try_from(value).map_err(|_| {
                Self::error(
                    "E104",
                    "list extent must be a non-negative target-sized constant integer",
                    expr.span,
                )
            });
        }
        self.scalar_input(expr)?;
        self.extent_value(expr)
    }

    pub(crate) fn extent_value(&mut self, expr: &ast::Expr) -> Result<usize> {
        let reach = std::mem::replace(&mut self.reach, crate::flow::TRUE);
        let required = std::mem::replace(&mut self.required, true);
        let result = self.expression(expr, None);
        self.reach = reach;
        self.required = required;
        let value = result?;
        match self.constant(&value) {
            Some(Constant::Int(value)) => usize::try_from(value).map_err(|_| {
                Self::error(
                    "E104",
                    "list extent must be a non-negative target-sized constant integer",
                    expr.span,
                )
            }),
            _ => Err(Self::error(
                "E104",
                "list extent must be a compile-time constant integer",
                expr.span,
            )),
        }
    }

    pub(crate) fn list_type(&mut self, element: Type, capacity: usize, span: Span) -> Result<Type> {
        if capacity > MAX_CAPACITY {
            return Err(Diagnostic::unsupported(
                "bounded-list capacity exceeds bootstrap budget",
                span,
            ));
        }
        let mut pending = vec![&element];
        while let Some(ty) = pending.pop() {
            if !self.flow.spend(1) {
                return Err(Diagnostic::unsupported("list type budget exhausted", span));
            }
            match ty {
                Type::Reference(_) | Type::Exclusive(_) | Type::Never => {
                    return Err(Diagnostic::unsupported(
                        "reference-bearing or uninhabited list elements",
                        span,
                    ));
                }
                Type::Record { primary, fields } => {
                    pending.push(primary);
                    for field in fields {
                        if pending.len() >= 4096 {
                            return Err(Diagnostic::unsupported(
                                "list type budget exhausted",
                                span,
                            ));
                        }
                        pending.push(&field.ty);
                    }
                }
                Type::Union(members) => {
                    for ty in members {
                        if pending.len() >= 4096 {
                            return Err(Diagnostic::unsupported(
                                "list type budget exhausted",
                                span,
                            ));
                        }
                        pending.push(ty);
                    }
                }
                Type::List { element, .. } => pending.push(element),
                _ => {}
            }
        }
        let ty = Type::List {
            element: Box::new(element),
            capacity,
        };
        let (size, _) = ty.layout().ok_or_else(|| {
            Self::error(
                "E104",
                "bounded-list layout overflows the target address space",
                span,
            )
        })?;
        if size > MAX_BYTES {
            return Err(Diagnostic::unsupported(
                "bounded-list layout exceeds bootstrap byte budget",
                span,
            ));
        }
        Ok(ty)
    }

    pub(crate) fn scalar_literal(&self, expr: &ast::Expr) -> bool {
        match &expr.kind {
            ExprKind::Int(_) | ExprKind::Float(_) => true,
            ExprKind::String(parts) => parts
                .iter()
                .all(|part| matches!(part, ast::StringPart::Text(_))),
            ExprKind::Group(value) => self.scalar_literal(value),
            ExprKind::Unary { op, value } if op == "-" => {
                matches!(value.kind, ExprKind::Int(_) | ExprKind::Float(_))
            }
            ExprKind::Name(name) => self
                .scopes
                .iter()
                .rev()
                .find_map(|scope| scope.values.get(name))
                .is_some_and(|value| matches!(value, Value::Constant(_))),
            _ => false,
        }
    }

    pub(crate) fn list_literal(
        &mut self,
        values: &[ast::Expr],
        expected: Option<&Type>,
        span: Span,
    ) -> Result<hir::Expr> {
        let contexts: Vec<_> = expected
            .into_iter()
            .flat_map(Type::members)
            .filter(|ty| matches!(ty, Type::List { .. }))
            .collect();
        if contexts.len() > 1 {
            return self.list_union(values, &contexts, span);
        }
        if let Some(Type::List { element, capacity }) = contexts.first().copied() {
            if values.len() > *capacity {
                return Err(Self::error(
                    "E103",
                    format!(
                        "list has {} elements but capacity is {capacity}",
                        values.len()
                    ),
                    span,
                ));
            }
            let list = self.list_type(*element.clone(), *capacity, span)?;
            let values = values
                .iter()
                .map(|value| self.expr(value, Some(element)))
                .collect::<Result<Vec<_>>>()?;
            let ty = if values.iter().any(|value| value.ty == Type::Never) {
                Type::Never
            } else {
                list.clone()
            };
            return Ok(hir::Expr {
                kind: hir::ExprKind::List { values, list },
                ty,
                span,
            });
        }
        if values.is_empty() {
            return Err(Self::error(
                "E207",
                "empty list literal requires an expected element type",
                span,
            ));
        }
        if values.len() > MAX_CAPACITY {
            return Err(Diagnostic::unsupported(
                "bounded-list capacity exceeds bootstrap budget",
                span,
            ));
        }
        let mut common = None;
        let mut items = vec![None; values.len()];
        let mut deferred = Vec::new();
        for (index, value) in values.iter().enumerate() {
            if self.scalar_literal(value) {
                deferred.push((index, self.reach));
                continue;
            }
            let value = self.expr(value, None)?;
            if value.ty != Type::Never {
                if common.as_ref().is_some_and(|ty| ty != &value.ty) {
                    return Err(Self::error(
                        "E207",
                        "already typed list elements require one identical normalized type",
                        value.span,
                    ));
                }
                common = Some(value.ty.clone());
            }
            items[index] = Some(value);
        }
        let reached = self.reach;
        for (index, reach) in deferred {
            self.reach = reach;
            let value = self.expr(&values[index], common.as_ref())?;
            if common.is_none() {
                common = Some(value.ty.clone());
            }
            items[index] = Some(value);
        }
        self.reach = reached;
        let element = common.unwrap_or(Type::Null);
        let list = self.list_type(element, values.len(), span)?;
        let values: Vec<_> = items
            .into_iter()
            .map(|value| value.expect("checked list element"))
            .collect();
        let ty = if values.iter().any(|value| value.ty == Type::Never) {
            Type::Never
        } else {
            list.clone()
        };
        Ok(hir::Expr {
            kind: hir::ExprKind::List { values, list },
            ty,
            span,
        })
    }

    pub(crate) fn literal_default(&self, value: &ast::Expr) -> Option<Type> {
        match &value.kind {
            ExprKind::Int(_) => Some(Type::Int {
                bits: 32,
                signed: true,
            }),
            ExprKind::Float(_) => Some(Type::Float { bits: 64 }),
            ExprKind::String(_) => Some(Type::String),
            ExprKind::Group(value) | ExprKind::Unary { value, .. } => self.literal_default(value),
            ExprKind::Name(name) => self
                .scopes
                .iter()
                .rev()
                .find_map(|scope| scope.values.get(name))
                .and_then(|symbol| {
                    if let Value::Constant(data) = symbol {
                        Some(Self::constant_expr(data.clone(), value.span).ty)
                    } else {
                        None
                    }
                }),
            _ => None,
        }
    }

    pub(crate) fn list_hint(&mut self, values: &[ast::Expr]) -> Option<Type> {
        let mut common = None;
        for value in values {
            if self.scalar_literal(value) {
                continue;
            }
            let ty = self.hint(value)?;
            if ty == Type::Never {
                continue;
            }
            if common.as_ref().is_some_and(|common| common != &ty) {
                return None;
            }
            common = Some(ty);
        }
        if common.is_none() {
            for value in values {
                let ty = self.literal_default(value)?;
                if common.as_ref().is_some_and(|common| common != &ty) {
                    return None;
                }
                common = Some(ty);
            }
        }
        Some(Type::List {
            element: Box::new(common?),
            capacity: values.len(),
        })
    }

    pub(crate) fn list_fact(&self, value: &hir::Expr) -> Option<Fact> {
        if value.ty == Type::Never {
            return None;
        }
        match &value.kind {
            hir::ExprKind::List { values, list } => Some(Fact {
                ty: list.clone(),
                length: values.len(),
            }),
            hir::ExprKind::Local(id) => self.lengths.get(id).cloned(),
            hir::ExprKind::Block(block) => self.block_lengths.get(&block.id).cloned(),
            hir::ExprKind::Coerce { value: source } => self
                .list_fact(source)
                .filter(|fact| value.ty.accepts(&fact.ty)),
            hir::ExprKind::ListAdd { value, .. } => self.list_fact(value).and_then(|mut fact| {
                fact.length = fact.length.checked_add(1)?;
                Some(fact)
            }),
            _ => None,
        }
    }

    pub(crate) fn list_length(&self, value: &hir::Expr) -> Option<usize> {
        match &value.ty {
            Type::List { capacity: 0, .. } => Some(0),
            Type::List { .. } => self
                .list_fact(value)
                .filter(|fact| fact.ty == value.ty)
                .map(|fact| fact.length),
            _ => None,
        }
    }

    pub(crate) fn block_list_length(&mut self, frame: &Frame, ty: &Type) {
        if !matches!(ty, Type::List { .. }) {
            return;
        }
        let Some(writes) = frame.slots.get(&None) else {
            return;
        };
        let mut fact: Option<Fact> = None;
        for slot in writes {
            if !self.flow.overlap(slot.guard, self.reach) {
                continue;
            }
            let Some(next) = slot.list.as_ref().filter(|fact| &fact.ty == ty) else {
                return;
            };
            if fact.as_ref().is_some_and(|fact| fact.length != next.length) {
                return;
            }
            fact = Some(next.clone());
        }
        if let Some(fact) = fact {
            self.block_lengths.insert(frame.id, fact);
        }
    }

    pub(crate) fn list_receiver(&mut self, value: &ast::Expr) -> Result<hir::Expr> {
        let mut value = self.expr(value, None)?;
        if let Type::Reference(ty) = &value.ty
            && matches!(ty.as_ref(), Type::List { .. })
        {
            value = hir::Expr {
                ty: *ty.clone(),
                span: value.span,
                kind: hir::ExprKind::Deref(Box::new(value)),
            };
        }
        Ok(value)
    }

    pub(crate) fn list_index(
        &mut self,
        value: &ast::Expr,
        index: &ast::Expr,
        span: Span,
    ) -> Result<hir::Expr> {
        let value = self.list_receiver(value)?;
        if value.ty == Type::Never {
            return Ok(value);
        }
        let Type::List { element, capacity } = &value.ty else {
            return Err(Diagnostic::unsupported(
                "indexing outside bounded lists",
                span,
            ));
        };
        let ty = *element.clone();
        let capacity = *capacity;
        let length = self.list_length(&value);
        let index = self.list_position(index, length, capacity)?;
        let ty = if index.ty == Type::Never {
            Type::Never
        } else {
            ty
        };
        Ok(hir::Expr {
            kind: hir::ExprKind::ListIndex {
                value: Box::new(value),
                index: Box::new(index),
            },
            ty,
            span,
        })
    }

    pub(crate) fn list_position(
        &mut self,
        index: &ast::Expr,
        length: Option<usize>,
        capacity: usize,
    ) -> Result<hir::Expr> {
        let index = self.expr(index, None)?;
        if index.ty == Type::String {
            return Err(Diagnostic::unsupported(
                "named-list alias lookup",
                index.span,
            ));
        }
        if !matches!(index.ty, Type::Int { .. } | Type::Never) {
            return Err(Self::error(
                "E222",
                "list positions require an integer",
                index.span,
            ));
        }
        if self.reach != FALSE
            && let Some(Constant::Int(position)) = self.constant(&index)
            && (position < 1 || position > length.unwrap_or(capacity) as i128)
        {
            return Err(Self::error(
                "E101",
                format!(
                    "one-based position {position} exceeds initialized length {}",
                    length
                        .map(|length| length.to_string())
                        .unwrap_or_else(|| format!("at most {capacity}"))
                ),
                index.span,
            ));
        }
        Ok(index)
    }

    pub(crate) fn element_borrow(
        &mut self,
        value: &ast::Expr,
        index: &ast::Expr,
        span: Span,
    ) -> Result<hir::Expr> {
        if !self.flow.spend(
            value
                .span
                .end
                .saturating_sub(value.span.start)
                .saturating_add(1),
        ) {
            return Err(Diagnostic::unsupported(
                "list element place budget exhausted",
                span,
            ));
        }
        let mut form = value;
        while let ExprKind::Group(value) = &form.kind {
            form = value;
        }
        let hint = self.hint(value);
        let place = matches!(
            form.kind,
            ExprKind::Name(_) | ExprKind::Field { .. } | ExprKind::Index { .. }
        ) || matches!(&form.kind, ExprKind::Unary { op, .. } if op == "*");
        let value = if place && !matches!(hint, Some(Type::Reference(_) | Type::Never)) {
            self.borrowed(value, value.span)?
        } else {
            let value = self.expr(value, None)?;
            if matches!(value.ty, Type::Reference(_) | Type::Never) {
                value
            } else {
                self.temporary_borrow(value, span)?
            }
        };
        if value.ty == Type::Never {
            return Ok(value);
        }
        let Type::Reference(target) = &value.ty else {
            return Err(Diagnostic::unsupported(
                "borrowing elements of temporary storage",
                span,
            ));
        };
        crate::borrow_contract::type_weight(target, &mut self.flow, span)?;
        let Type::List { element, capacity } = target.as_ref() else {
            return Err(Diagnostic::unsupported(
                "element borrows outside bounded lists",
                span,
            ));
        };
        if element.has_reference() {
            return Err(Diagnostic::unsupported(
                "borrowing reference-bearing list elements",
                span,
            ));
        }
        let ty = Type::Reference(element.clone());
        let capacity = *capacity;
        let length = self.borrowed_list_length(&value);
        let index = self.list_position(index, length, capacity)?;
        let ty = if index.ty == Type::Never {
            Type::Never
        } else {
            ty
        };
        let site = self.reborrows;
        self.reborrows += 1;
        Ok(hir::Expr {
            kind: hir::ExprKind::ElementBorrow {
                site,
                value: Box::new(value),
                index: Box::new(index),
            },
            ty,
            span,
        })
    }

    pub(crate) fn borrowed_list_length(&self, value: &hir::Expr) -> Option<usize> {
        match &value.kind {
            hir::ExprKind::TemporaryBorrow { value, .. } => {
                self.list_fact(value).map(|fact| fact.length)
            }
            hir::ExprKind::Borrow(place) if place.fields.is_empty() => {
                self.lengths.get(&place.root).map(|fact| fact.length)
            }
            hir::ExprKind::Reborrow { value, fields, .. } if fields.is_empty() => {
                self.borrowed_list_length(value)
            }
            _ => None,
        }
    }

    pub(crate) fn list_method(
        &mut self,
        value: &ast::Expr,
        name: &str,
        args: &[ast::Expr],
        span: Span,
    ) -> Result<hir::Expr> {
        let value = self.list_receiver(value)?;
        if value.ty == Type::Never {
            return Ok(value);
        }
        if name == "size" {
            if !args.is_empty() {
                return Err(Self::error("E212", "size takes no arguments", span));
            }
            let kind = match value.ty {
                Type::List { .. } => hir::ExprKind::ListSize(Box::new(value)),
                Type::String => hir::ExprKind::StringSize(Box::new(value)),
                _ => return Err(Self::error("E201", "size requires a list or string", span)),
            };
            return Ok(hir::Expr {
                kind,
                ty: Type::Int {
                    bits: 64,
                    signed: false,
                },
                span,
            });
        }
        let Type::List { element, capacity } = &value.ty else {
            return Err(Self::error("E201", "add requires a bounded list", span));
        };
        if args.len() != 1 {
            return Err(Self::error("E212", "list.add takes one element", span));
        }
        let length = self.list_length(&value);
        let capacity = *capacity;
        let item = self.expr(&args[0], Some(element))?;
        if self.reach != FALSE && length == Some(capacity) {
            return Err(Self::error(
                "E103",
                format!("list is full at capacity {capacity}"),
                span,
            ));
        }
        let ty = if item.ty == Type::Never {
            Type::Never
        } else {
            value.ty.clone()
        };
        Ok(hir::Expr {
            kind: hir::ExprKind::ListAdd {
                value: Box::new(value),
                item: Box::new(item),
            },
            ty,
            span,
        })
    }

    pub(crate) fn value_formattable(ty: &Type) -> bool {
        match ty {
            Type::List { .. } | Type::Foundation(_) => false,
            Type::Record { primary, .. } => Self::value_formattable(primary),
            Type::Union(members) => members.iter().all(Self::value_formattable),
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub(crate) fn element_assignment_checks_storage_index_and_context_in_order() {
        for source in [
            "a:=[1,2];a[1]=3",
            "a<uint8[2]>:=[1,2];(a)[1]=255",
            "a:=[[1,2],[3,4]];a[1]=[5,6]",
            "a:=[{->n:1},{->n:2}];a[1]={->n:3}",
            "<Item>:<int32><string>;a<Item[2]>:=[1,2];a[1]=\"x\"",
            "<Item>:<int32><string>;a<Item[2]>:=[1,2];copy:a[1];a[1]=\"x\";|copy<int32>|{number<int32>:copy}",
            "d:@\"debug\";a:=[1,2];a[{d.panic(\"stop\")}]=1/0",
        ] {
            let result = crate::compile(source);
            assert!(result.is_ok(), "{source}: {result:?}");
        }
        for (source, code) in [
            ("a:[1,2];a[1]=3", "E305"),
            ("a:=[1,2];a[0]=3", "E101"),
            ("a:=[1,2];a[3]=3", "E101"),
            ("d:@\"debug\";a:=[1,2];a[3]=d.panic(\"stop\")", "E101"),
            ("a:=[1,2];a[true]=3", "E222"),
            ("a:=[1,2];a[1]=\"x\"", "E207"),
            ("a<uint8[2]>:=[1,2];a[1]=256", "E216"),
            ("a:=[1,2];p:&a;p[1]=3", "B001"),
            ("a:={->items:[1,2]};a.items[1]=3", "E305"),
            ("[1,2][1]=3", "B001"),
        ] {
            rejects(source, code);
        }
    }

    #[test]
    pub(crate) fn nested_assignment_paths_preserve_prefix_spans_and_leaf_context() {
        for source in [
            "a:=[[1,2],[3,4]];a[1][2]=5",
            "a<uint8[2][2]>:=[[1,2],[3,4]];((a)[1])[2]=255",
            "a:=[[[1,2]],[[3,4]]];a[2][1]=[5,6]",
            "a:=[[{->n:1}],[{->n:2}]];a[2][1]={->n:3}",
        ] {
            let result = crate::compile(source);
            assert!(result.is_ok(), "{source}: {result:?}");
        }
        for (source, code) in [
            ("a:=[[1,2],[3,4]];a[1][0]=5", "E101"),
            ("a:=[[1,2],[3,4]];a[0][1]=5", "E101"),
            ("a<uint8[2][2]>:=[[1,2],[3,4]];a[1][2]=256", "E216"),
            ("a:=[[1,2],[3,4]];a[1][2]=\"x\"", "E207"),
            ("a:=[{->items:[1,2]}];a[1].items[2]=5", "E305"),
            ("a:=[[1,2],[3,4]];r:&a;r[1][2]=5", "B001"),
        ] {
            rejects(source, code);
        }
        let source = "a:=[[1,2],[3,4]];a[1][2]=5";
        let program = crate::compile(source).unwrap();
        let Some(crate::hir::Stmt::SetPath { path, .. }) = program.body.stmts.last() else {
            panic!("expected initialized assignment");
        };
        let prefixes = path
            .iter()
            .map(|step| {
                let crate::hir::WriteStep::Index(step) = step else {
                    panic!("index step")
                };
                &source[step.span.start..step.span.end]
            })
            .collect::<Vec<_>>();
        assert_eq!(prefixes, ["a[1]", "a[1][2]"]);
        for depth in [32, 64] {
            let mut value = "1".to_owned();
            for _ in 0..depth {
                value = format!("[{value}]");
            }
            let source = format!("a:={value};a{}=2", "[1]".repeat(depth));
            if depth == 32 {
                let result = crate::compile(&source);
                assert!(result.is_ok(), "{result:?}");
            } else {
                rejects(&source, "B001");
            }
        }
    }

    #[test]
    pub(crate) fn element_places_and_temporaries_share_position_checks() {
        for source in [
            "a:[1,2];r:&(a[1]);value:*r",
            "a:[[1,2],[3,4]];r:&(a[2][1]);value:*r",
            "a:[{->value:1},{->value:2}];r:&(a[1].value);value:*r",
            "a:[[1,2],[3,4]];r:&(({->&a})[2][1]);value:*r",
            "a:[1,2];p:&a;r:&((*p)[1]);value:*r",
            "a:[1,2];holder:{->view:&a};r:&(holder.view[1]);value:*r",
            "get<&int32>:(a<&int32[0]>,i<int32>){->&(a[i])}",
            "value:*(&([1,2][1]))",
            "make<int32[2]>:(){->[1,2]};value:*(&(make()[1]))",
            "a:[1,2];value:*(&(a<int32[2]>[1]))",
        ] {
            let result = crate::compile(source);
            assert!(result.is_ok(), "{source}: {result:?}");
        }
        for source in [
            "a:[1,2];r:&(a[0])",
            "a:[1,2];r:&(a[-1])",
            "a<int32[3]>:[1];r:&(a[2])",
            "a<int32[0]>:[];r:&(a[1])",
        ] {
            rejects(source, "E101");
        }
        rejects("a:[1,2];r:&!(a[1])", "E305");
        rejects("a:[1,2];r:&(a[true])", "E222");
    }

    pub(crate) fn rejects(source: &str, code: &str) {
        let errors = crate::compile(source).unwrap_err();
        assert_eq!(errors[0].code, code, "{source}: {errors:?}");
    }

    #[test]
    pub(crate) fn required_extents_preserve_width_checks_even_on_dead_paths() {
        rejects(
            "capacity<uint8>:255;|false|{values<int32[capacity+1]>:[]}",
            "E107",
        );
        rejects("a<uint8>:2;b<uint16>:2;values<int32[a+b]>:[]", "E213");
        rejects("capacity:=2;f<null>:(){values<int32[capacity]>:[]}", "E104");
        rejects("values<int32[-1]>:[]", "E104");
        rejects("capacity:(){->2};values<int32[capacity()]>:[]", "B001");
    }

    #[test]
    pub(crate) fn later_typed_values_contextualize_only_pure_scalar_literals() {
        assert!(crate::compile("byte<uint8>:1;values:[2,byte]").is_ok());
        assert!(crate::compile("wide<uint64>:1;values:[4294967295,wide]").is_ok());
        rejects("byte<uint8>:1;values:[byte,1+1]", "E207");
        rejects("byte<uint8>:1;values:[byte,{->2}]", "E207");
        rejects("record:{->1;->tag:true};values:[record,2]", "E207");
        assert!(crate::compile("values<int32[1]><string[1]>:[1]").is_ok());
    }

    #[test]
    pub(crate) fn list_budget_and_unimplemented_boundaries_are_explicit() {
        rejects("values<int32[65537]>:[]", "B001");
        rejects("values<string[65536]>:[]", "B001");
        rejects("a:1;values:[&a]", "B001");
        rejects("values:[\"name\":1]", "B001");
        rejects("d:@\"debug\";values:[1];d.print(values)", "B001");
        rejects("values:[1];view:&!(values[1])", "E305");
    }
}
