use crate::ast::Span;

pub type LocalId = usize;
pub type FunctionId = usize;
pub type BlockId = usize;
pub type EmitId = usize;
pub type CallId = usize;
pub type ReborrowId = usize;
pub type StatementId = usize;
pub type PointId = usize;
pub type RestartId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FoundationType {
    Allocator,
    AllocationFailure,
    OwnedString,
}

impl FoundationType {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Allocator => "memory.Allocator",
            Self::AllocationFailure => "memory.AllocationFailure",
            Self::OwnedString => "strings.Owned",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReferenceMode {
    Shared,
    Exclusive,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
    Null,
    Never,
    Bool,
    Int {
        bits: u32,
        signed: bool,
    },
    Float {
        bits: u32,
    },
    String,
    Foundation(FoundationType),
    List {
        element: Box<Type>,
        capacity: usize,
    },
    Reference(Box<Type>),
    Exclusive(Box<Type>),
    Record {
        primary: Box<Type>,
        fields: Vec<Field>,
    },
    Union(Vec<Type>),
}

impl Type {
    pub(crate) fn has_mutable_fields(&self) -> bool {
        match self {
            Self::Record { primary, fields } => {
                primary.has_mutable_fields()
                    || fields
                        .iter()
                        .any(|field| field.mutable || field.ty.has_mutable_fields())
            }
            Self::List { element, .. } => element.has_mutable_fields(),
            Self::Union(types) => types.iter().any(Self::has_mutable_fields),
            _ => false,
        }
    }

    pub fn pointee(&self) -> Option<&Type> {
        match self {
            Self::Reference(ty) | Self::Exclusive(ty) => Some(ty),
            _ => None,
        }
    }

    pub fn reference_mode(&self) -> Option<ReferenceMode> {
        match self {
            Self::Reference(_) => Some(ReferenceMode::Shared),
            Self::Exclusive(_) => Some(ReferenceMode::Exclusive),
            _ => None,
        }
    }

    pub fn has_exclusive(&self) -> bool {
        match self {
            Self::Exclusive(_) => true,
            Self::Reference(ty) | Self::List { element: ty, .. } => ty.has_exclusive(),
            Self::Record { primary, fields } => {
                primary.has_exclusive() || fields.iter().any(|field| field.ty.has_exclusive())
            }
            Self::Union(types) => types.iter().any(Self::has_exclusive),
            _ => false,
        }
    }

    pub fn is_copy(&self) -> bool {
        match self {
            Self::Null
            | Self::Never
            | Self::Bool
            | Self::Int { .. }
            | Self::Float { .. }
            | Self::String
            | Self::Reference(_) => true,
            Self::Exclusive(_) => false,
            Self::Foundation(ty) => *ty != FoundationType::OwnedString,
            Self::List { element, .. } => element.is_copy(),
            Self::Record { primary, fields } => {
                primary.is_copy() && fields.iter().all(|field| field.ty.is_copy())
            }
            Self::Union(types) => types.iter().all(Self::is_copy),
        }
    }

    pub fn has_drop(&self) -> bool {
        match self {
            Self::Foundation(FoundationType::OwnedString) => true,
            Self::Record { primary, fields } => {
                primary.has_drop() || fields.iter().any(|field| field.ty.has_drop())
            }
            Self::List { element, .. } => element.has_drop(),
            Self::Union(types) => types.iter().any(Self::has_drop),
            _ => false,
        }
    }

    pub fn has_allocator_value(&self) -> bool {
        match self {
            Self::Foundation(FoundationType::Allocator) => true,
            Self::Record { primary, fields } => {
                primary.has_allocator_value()
                    || fields.iter().any(|field| field.ty.has_allocator_value())
            }
            Self::List { element, .. } => element.has_allocator_value(),
            Self::Union(types) => types.iter().any(Self::has_allocator_value),
            _ => false,
        }
    }

    pub(crate) fn fixed_borrowed_part(&self) -> bool {
        match self {
            Self::Reference(ty) => ty.fixed_borrowed_part(),
            Self::Exclusive(_) | Self::List { .. } => false,
            Self::Union(types) => types.iter().all(Self::fixed_borrowed_part),
            Self::Record { primary, fields } => {
                primary.fixed_borrowed_part()
                    && fields.iter().all(|field| field.ty.fixed_borrowed_part())
            }
            _ => self.is_copy(),
        }
    }

    pub(crate) fn fixed_borrowed_value(&self) -> bool {
        self.has_borrowed() && self.fixed_borrowed_part()
    }

    pub fn has_borrowed(&self) -> bool {
        self.has_reference() || self.has_allocator_value()
    }

    pub fn has_equality(&self) -> bool {
        match self {
            Self::Foundation(_) => false,
            Self::Record { primary, fields } => {
                primary.has_equality() && fields.iter().all(|field| field.ty.has_equality())
            }
            Self::List { element, .. } => element.has_equality(),
            Self::Union(types) => types.iter().all(Self::has_equality),
            _ => true,
        }
    }

    pub fn layout(&self) -> Option<(usize, usize)> {
        let ty = self;
        match ty {
            Type::Null | Type::Never | Type::Bool => Some((1, 1)),
            Type::Int { bits, .. } | Type::Float { bits } => {
                let size = (*bits as usize).div_ceil(8);
                Some((size, size))
            }
            Type::String => Some((16, 8)),
            Type::Foundation(FoundationType::Allocator) => Some((8, 8)),
            Type::Foundation(FoundationType::AllocationFailure | FoundationType::OwnedString) => {
                Some((24, 8))
            }
            Type::Reference(_) | Type::Exclusive(_) => Some((8, 8)),
            Type::Record { primary, fields } => {
                let mut size = 0usize;
                let mut align = 1;
                for ty in
                    std::iter::once(primary.as_ref()).chain(fields.iter().map(|field| &field.ty))
                {
                    let (part, boundary) = ty.layout()?;
                    size = size.checked_next_multiple_of(boundary)?.checked_add(part)?;
                    align = align.max(boundary);
                }
                Some((size.checked_next_multiple_of(align)?, align))
            }
            Type::Union(members) => {
                let mut size = 0;
                for member in members {
                    size = size.max(member.layout()?.0);
                }
                Some((8usize.checked_add(size.checked_next_multiple_of(8)?)?, 8))
            }
            Type::List { element, capacity } => {
                let (size, align) = element.layout()?;
                let start = 8usize.checked_next_multiple_of(align)?;
                let size = start.checked_add(size.checked_mul(*capacity)?)?;
                let align = align.max(8);
                Some((size.checked_next_multiple_of(align)?, align))
            }
        }
    }

    pub fn has_reference(&self) -> bool {
        match self {
            Self::List { element, .. } => element.has_reference(),
            Self::Reference(_) | Self::Exclusive(_) => true,
            Self::Record { primary, fields } => {
                primary.has_reference() || fields.iter().any(|field| field.ty.has_reference())
            }
            Self::Union(types) => types.iter().any(Self::has_reference),
            _ => false,
        }
    }

    pub fn union(types: impl IntoIterator<Item = Type>) -> Self {
        let mut members = Vec::new();
        for ty in types {
            match ty {
                Self::Never => {}
                Self::Union(types) => members.extend(Self::union(types).members().iter().cloned()),
                ty => members.push(ty),
            }
        }
        members.sort();
        members.dedup();
        match members.len() {
            0 => Self::Never,
            1 => members.pop().expect("one union member"),
            _ => Self::Union(members),
        }
    }

    pub fn members(&self) -> &[Self] {
        match self {
            Self::Never => &[],
            Self::Union(types) => types,
            ty => std::slice::from_ref(ty),
        }
    }

    pub fn accepts(&self, value: &Self) -> bool {
        value.members().iter().all(|ty| self.members().contains(ty))
    }

    pub fn intersection(&self, other: &Self) -> Self {
        Self::union(
            self.members()
                .iter()
                .filter(|ty| other.members().contains(ty))
                .cloned(),
        )
    }

    pub fn subtract(&self, other: &Self) -> Self {
        Self::union(
            self.members()
                .iter()
                .filter(|ty| !other.members().contains(ty))
                .cloned(),
        )
    }
}

#[derive(Clone, Debug)]
pub struct Program {
    pub body: Block,
    pub functions: Vec<Function>,
    pub locals: Vec<Type>,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub id: FunctionId,
    pub name: String,
    pub params: Vec<LocalId>,
    pub result: Type,
    pub body: Block,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub id: BlockId,
    pub ty: Type,
    pub stmts: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub struct IndexStep {
    pub index: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum WriteStep {
    Field(usize),
    Index(IndexStep),
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Statement {
        id: StatementId,
        stmts: Vec<Stmt>,
    },
    Bind {
        id: LocalId,
        value: Expr,
    },
    SlotAlias {
        id: LocalId,
        target: BlockId,
        field: String,
        mutable: bool,
    },
    Assign {
        id: LocalId,
        value: Expr,
    },
    SetPath {
        id: LocalId,
        path: Vec<WriteStep>,
        value: Expr,
        span: Span,
    },
    Store {
        target: Expr,
        value: Expr,
        span: Span,
    },
    Emit {
        id: EmitId,
        target: BlockId,
        field: Option<String>,
        value: Expr,
    },
    If {
        point: Option<PointId>,
        condition: Expr,
        then: Vec<Stmt>,
        otherwise: Vec<Stmt>,
    },
    Leave(BlockId),
    Restart {
        target: BlockId,
        site: RestartId,
    },
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: Type,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub root: LocalId,
    pub fields: Vec<usize>,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Null,
    Bool(bool),
    Int(i128),
    Float(f64),
    String(String),
    Heap,
    List {
        values: Vec<Expr>,
        list: Type,
    },
    ListSize(Box<Expr>),
    ListIndex {
        value: Box<Expr>,
        index: Box<Expr>,
    },
    ListAdd {
        value: Box<Expr>,
        item: Box<Expr>,
    },
    Local(LocalId),
    Borrow(Place),
    TemporaryBorrow {
        id: LocalId,
        statement: StatementId,
        value: Box<Expr>,
    },
    Reborrow {
        site: ReborrowId,
        value: Box<Expr>,
        fields: Vec<usize>,
    },
    ExclusivePath {
        place: Place,
        path: Vec<WriteStep>,
    },
    ElementBorrow {
        site: ReborrowId,
        value: Box<Expr>,
        index: Box<Expr>,
    },
    Deref(Box<Expr>),
    Unary {
        op: String,
        value: Box<Expr>,
    },
    Binary {
        point: Option<PointId>,
        op: String,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        id: FunctionId,
        site: CallId,
        args: Vec<Expr>,
    },
    Print {
        parts: Vec<Expr>,
        newline: bool,
    },
    Panic {
        parts: Vec<Expr>,
    },
    Block(Block),
    Field {
        value: Box<Expr>,
        index: usize,
    },
    Primary(Box<Expr>),
    StringSize(Box<Expr>),
    Coerce {
        value: Box<Expr>,
    },
    TypeTest {
        value: Box<Expr>,
        ty: Type,
    },
}

#[cfg(test)]
mod tests {
    use super::Type;

    #[test]
    pub(crate) fn unions_are_normalized_sets_with_null_first() {
        let ty = Type::union([
            Type::String,
            Type::Never,
            Type::Null,
            Type::union([Type::Bool, Type::String]),
        ]);
        assert_eq!(ty, Type::Union(vec![Type::Null, Type::Bool, Type::String]));
        assert_eq!(
            Type::union([Type::Never, Type::String, Type::String]),
            Type::String
        );
        assert_eq!(Type::union([]), Type::Never);
        assert!(ty.accepts(&Type::union([Type::String, Type::Null])));
        assert!(!Type::String.accepts(&ty));
        assert_eq!(
            ty.subtract(&Type::Null),
            Type::union([Type::Bool, Type::String])
        );
        assert_eq!(ty.intersection(&Type::String), Type::String);
    }
}
