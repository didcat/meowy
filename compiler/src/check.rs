mod aliases;
mod blocks;
mod documentation;
mod exports;
mod expressions;
mod functions;
mod indexed;
mod inputs;
mod mutation;
mod names;
mod references;
mod refinement;
mod scalars;
mod statements;
mod temporaries;
mod type_values;

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{self, Span};
use crate::diagnostic::Diagnostic;
use crate::flow::{Flow, Guard, TRUE};
use crate::hir::{self, Type};

pub(crate) type Result<T> = std::result::Result<T, Diagnostic>;
pub(crate) type Slots = BTreeMap<Option<String>, Vec<Slot>>;
pub(crate) type Place = (usize, Vec<String>);

#[derive(Clone)]
pub(crate) enum Constant {
    Null,
    Bool(bool),
    Int(i128),
    Float(f64),
    String(String),
}

#[derive(Clone)]
pub(crate) enum Value {
    Local {
        id: usize,
        ty: Type,
        mutable: bool,
        owner: usize,
        constant: Option<Constant>,
    },
    Constant(Constant),
    Static {
        value: Constant,
        ty: Type,
    },
    Record {
        ty: Type,
        input: Box<inputs::Record>,
    },
    Module(crate::foundation::Module),
    FileModule {
        id: usize,
        ty: Type,
    },
    Foundation(crate::foundation::Item),
    Function {
        id: usize,
        params: Vec<Type>,
        result: Option<Type>,
    },
    Print,
    Panic,
    Control {
        target: usize,
        restart: bool,
        owner: usize,
    },
    Type(Type),
}

impl Value {
    pub(crate) fn data_type(&self) -> Option<Type> {
        match self {
            Self::Local { ty, .. }
            | Self::Static { ty, .. }
            | Self::FileModule { ty, .. }
            | Self::Record { ty, .. } => Some(ty.clone()),
            Self::Constant(value) => {
                Some(Checker::constant_expr(value.clone(), crate::ast::Span::new(0, 0)).ty)
            }
            _ => None,
        }
    }
}

#[derive(Clone)]
pub(crate) enum Spec {
    Meta,
    Descriptor(crate::foundation::Descriptor),
    Data(Type),
    Function { params: Vec<Type>, result: Type },
}

#[derive(Default)]
pub(crate) struct Scope {
    pub(crate) values: BTreeMap<String, Value>,
    pub(crate) types: BTreeMap<String, Spec>,
    pub(crate) labels: BTreeMap<String, usize>,
    pub(crate) doc_values: BTreeMap<String, usize>,
    pub(crate) doc_types: BTreeMap<String, usize>,
}

#[derive(Clone)]
pub(crate) struct Slot {
    pub(crate) ty: Type,
    pub(crate) mutable: bool,
    pub(crate) guard: Guard,
    pub(crate) order: usize,
    pub(crate) list: Option<crate::list::Fact>,
}

pub(crate) struct Frame {
    pub(crate) id: usize,
    pub(crate) expected: Option<Type>,
    pub(crate) leaves: Guard,
    pub(crate) slots: Slots,
    pub(crate) start: usize,
    pub(crate) first: hir::EmitId,
    pub(crate) partial: bool,
    pub(crate) owner: usize,
}

pub(crate) struct Checker {
    pub(crate) scopes: Vec<Scope>,
    pub(crate) frames: Vec<Frame>,
    pub(crate) flow: Flow,
    pub(crate) reach: Guard,
    pub(crate) tags: BTreeMap<(Place, Type), Vec<(Type, Guard)>>,
    pub(crate) bools: BTreeMap<Place, Guard>,
    pub(crate) guards: BTreeMap<(usize, usize), Guard>,
    pub(crate) writes: usize,
    pub(crate) functions: Vec<Option<hir::Function>>,
    pub(crate) locals: Vec<Type>,
    pub(crate) places: BTreeSet<usize>,
    pub(crate) proofs: crate::borrow::Proofs,
    pub(crate) constants: BTreeMap<usize, Constant>,
    pub(crate) inputs: BTreeMap<usize, inputs::Input>,
    pub(crate) bool_inputs: BTreeMap<usize, inputs::Input<bool>>,
    pub(crate) record_inputs: BTreeMap<usize, inputs::Record>,
    pub(crate) block: usize,
    pub(crate) owner: usize,
    pub(crate) calls: usize,
    pub(crate) reborrows: usize,
    pub(crate) restarts: hir::RestartId,
    pub(crate) statements: usize,
    pub(crate) statement: Vec<(hir::StatementId, bool)>,
    pub(crate) lengths: BTreeMap<usize, crate::list::Fact>,
    pub(crate) block_lengths: BTreeMap<usize, crate::list::Fact>,
    pub(crate) required: bool,
    pub(crate) type_work: Option<type_values::Work>,
    pub(crate) documentation: Option<crate::documentation::Model>,
    pub(crate) imports: BTreeMap<usize, String>,
    pub(crate) file_docs: BTreeMap<usize, crate::documentation::Model>,
    pub(crate) exports: BTreeMap<usize, exports::Module>,
    pub(crate) module: exports::Module,
}

pub fn check(block: &ast::Block) -> std::result::Result<hir::Program, Vec<Diagnostic>> {
    check_documented(block, None).map(|(program, _)| program)
}

pub(crate) fn check_documented(
    block: &ast::Block,
    docs: Option<crate::documentation::Model>,
) -> std::result::Result<(hir::Program, Option<crate::documentation::Model>), Vec<Diagnostic>> {
    check_imports(block, docs, BTreeMap::new(), BTreeMap::new())
}

pub(crate) fn check_imports(
    block: &ast::Block,
    docs: Option<crate::documentation::Model>,
    imports: BTreeMap<usize, String>,
    file_docs: BTreeMap<usize, crate::documentation::Model>,
) -> std::result::Result<(hir::Program, Option<crate::documentation::Model>), Vec<Diagnostic>> {
    let mut checker = Checker::new();
    checker.documentation = docs;
    checker.imports = imports;
    checker.file_docs = file_docs;
    match checker.block(block, None, None) {
        Ok(body) => {
            let program = hir::Program {
                body,
                functions: checker.functions.into_iter().flatten().collect(),
                locals: checker.locals,
            };
            checker.proofs.conditions = checker.guards;
            checker.proofs.tags = checker.tags;
            for function in &program.functions {
                if (function.result.has_exclusive()
                    || function
                        .params
                        .iter()
                        .any(|id| program.locals[*id].has_exclusive()))
                    && !(crate::borrow_contract::scalar_call(
                        &function.result,
                        function.params.iter().map(|id| &program.locals[*id]),
                    ) || crate::borrow_contract::reference_call(
                        &function.result,
                        function.params.iter().map(|id| &program.locals[*id]),
                    ))
                {
                    return Err(vec![Diagnostic::unsupported(
                        "exclusive function signatures outside scalar arguments and results",
                        Span::default(),
                    )]);
                }
            }
            crate::borrow::carried::validate(&checker.proofs, &mut checker.flow)
                .map_err(|error| vec![error])?;
            let facts = crate::borrow::check(&program, &mut checker.flow, &checker.proofs)?;
            crate::loans::check(&program, &facts, &checker.proofs, &mut checker.flow)?;
            let mut docs = checker.documentation;
            if let Some(model) = &mut docs {
                model.finish().map_err(|error| vec![error])?;
            }
            Ok((program, docs))
        }
        Err(error) => Err(vec![if checker.flow.exceeded() {
            Diagnostic::unsupported("control-flow proof budget exhausted", block.span)
        } else {
            error
        }]),
    }
}

impl Checker {
    pub(crate) fn new() -> Self {
        let mut prelude = Scope::default();
        prelude
            .values
            .insert("true".into(), Value::Constant(Constant::Bool(true)));
        prelude
            .values
            .insert("false".into(), Value::Constant(Constant::Bool(false)));
        prelude
            .values
            .insert("null".into(), Value::Constant(Constant::Null));
        for name in [
            "null", "never", "boolean", "int8", "int16", "int32", "int64", "uint8", "uint16",
            "uint32", "uint64", "isize", "usize", "float32", "float64", "string",
        ] {
            if let Some(ty) = Self::primitive(name) {
                prelude.types.insert(name.into(), Spec::Data(ty));
            }
        }
        prelude.types.insert("Type".into(), Spec::Meta);
        Self {
            scopes: vec![prelude],
            frames: Vec::new(),
            flow: Flow::new(),
            reach: TRUE,
            tags: BTreeMap::new(),
            bools: BTreeMap::new(),
            guards: BTreeMap::new(),
            writes: 0,
            functions: Vec::new(),
            locals: Vec::new(),
            places: BTreeSet::new(),
            proofs: crate::borrow::Proofs::default(),
            constants: BTreeMap::new(),
            inputs: BTreeMap::new(),
            bool_inputs: BTreeMap::new(),
            record_inputs: BTreeMap::new(),
            block: 0,
            owner: 0,
            calls: 0,
            reborrows: 0,
            restarts: 0,
            statements: 0,
            statement: Vec::new(),
            lengths: BTreeMap::new(),
            block_lengths: BTreeMap::new(),
            required: false,
            type_work: None,
            documentation: None,
            file_docs: BTreeMap::new(),
            imports: BTreeMap::new(),
            exports: BTreeMap::new(),
            module: exports::Module {
                depth: 2,
                ..exports::Module::default()
            },
        }
    }

    pub(crate) fn error(code: &'static str, message: impl Into<String>, span: Span) -> Diagnostic {
        Diagnostic::new(code, message, span)
    }
}

#[cfg(test)]
mod tests;
