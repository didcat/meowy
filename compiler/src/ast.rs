#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Clone, Debug)]
pub struct Block {
    pub label: Option<String>,
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Bind {
        name: String,
        ty: Option<TypeExpr>,
        mutable: bool,
        value: Expr,
    },
    Forward {
        name: String,
        ty: TypeExpr,
    },
    TypeAlias {
        name: String,
        ty: TypeExpr,
        exported: bool,
    },
    Assign {
        target: Expr,
        value: Expr,
    },
    Emit {
        label: Option<String>,
        name: Option<String>,
        ty: Option<TypeExpr>,
        mutable: bool,
        value: Expr,
    },
    Match {
        arms: Vec<(Option<Expr>, Box<Stmt>)>,
    },
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn spelling(&self) -> Option<&str> {
        match &self.kind {
            ExprKind::Name(name) => Some(name),
            ExprKind::Int(number) | ExprKind::Float(number) if !number.literal => Some(number),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Number {
    pub text: String,
    pub literal: bool,
}

impl Number {
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl std::ops::Deref for Number {
    type Target = str;

    fn deref(&self) -> &str {
        &self.text
    }
}

impl From<String> for Number {
    fn from(text: String) -> Self {
        Self {
            text,
            literal: false,
        }
    }
}

impl From<&str> for Number {
    fn from(text: &str) -> Self {
        text.to_owned().into()
    }
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Int(Number),
    Float(Number),
    String(Vec<StringPart>),
    Name(String),
    Import(String),
    Group(Box<Expr>),
    Unary {
        op: String,
        value: Box<Expr>,
    },
    Binary {
        op: String,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Field {
        value: Box<Expr>,
        name: String,
    },
    Index {
        value: Box<Expr>,
        index: Box<Expr>,
    },
    Block(Block),
    Function {
        params: Vec<Param>,
        body: Block,
    },
    Dispatch {
        value: Box<Expr>,
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    DispatchBlock {
        value: Box<Expr>,
        block: Block,
    },
    Ascribe {
        value: Box<Expr>,
        ty: TypeExpr,
        predicate: bool,
    },
    Specialize {
        value: Box<Expr>,
        types: Vec<TypeExpr>,
    },
    TypeQuery(Box<Expr>),
    TypeValue(TypeExpr),
    List(Vec<Expr>),
    Label(String),
    Unsupported(String),
}

#[derive(Clone, Debug)]
pub enum StringPart {
    Text(String),
    Value(Expr),
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TypeExpr {
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum TypeKind {
    Name(String),
    Union(Vec<TypeExpr>),
    Function {
        params: Vec<TypeExpr>,
        result: Box<TypeExpr>,
    },
    Record {
        primary: Option<Box<TypeExpr>>,
        fields: Vec<(String, TypeExpr, bool)>,
    },
    List {
        element: Box<TypeExpr>,
        size: Option<Box<Expr>>,
    },
    Reference {
        value: Box<TypeExpr>,
        mutable: bool,
    },
    Computed(Box<Expr>),
    Unsupported(String),
}
