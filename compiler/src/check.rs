mod aliases;
mod bits;
mod blocks;
mod dependencies;
mod documentation;
mod exports;
mod expressions;
mod functions;
mod indexed;
mod inputs;
mod mutation;
mod names;
mod queries;
mod references;
mod refinement;
mod required;
mod scalars;
mod statements;
mod temporaries;
mod type_values;

pub(crate) use dependencies::{ElementAccess, IndexAccess, MethodKind, PointKind, SequenceSource};

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
    Pending(usize),
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
    pub(crate) continuation: bool,
    pub(crate) id: usize,
    pub(crate) expected: Option<Type>,
    pub(crate) leaves: Guard,
    pub(crate) slots: Slots,
    pub(crate) start: usize,
    pub(crate) first: hir::EmitId,
    pub(crate) partial: bool,
    pub(crate) owner: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct InputUse {
    pub(crate) point: usize,
    pub(crate) site: Option<hir::StatementId>,
    pub(crate) id: hir::LocalId,
    pub(crate) storage: hir::LocalId,
    pub(crate) span: Span,
    pub(crate) control: bool,
    pub(crate) root: Span,
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
    pub(crate) derived: BTreeSet<usize>,
    pub(crate) pointees: BTreeMap<usize, dependencies::Origins>,
    pub(crate) reference_cells: BTreeMap<usize, dependencies::Cells>,
    pub(crate) record_cells: BTreeMap<usize, BTreeMap<Vec<usize>, dependencies::Cells>>,
    pub(crate) record_shapes: BTreeMap<usize, dependencies::Shapes>,
    pub(crate) record_compositions: BTreeMap<usize, Vec<usize>>,
    pub(crate) record_pointees: BTreeMap<usize, BTreeMap<Vec<usize>, dependencies::Origins>>,
    pub(crate) control: bool,
    pub(crate) inputs: BTreeMap<usize, inputs::Input>,
    pub(crate) bool_inputs: BTreeMap<usize, inputs::Input<bool>>,
    pub(crate) record_inputs: BTreeMap<usize, inputs::Record>,
    pub(crate) block: usize,
    pub(crate) owner: usize,
    pub(crate) calls: usize,
    pub(crate) invocations: BTreeMap<hir::CallId, dependencies::Invocation>,
    pub(crate) invocation_edges: usize,
    pub(crate) indices: BTreeMap<hir::PointId, dependencies::Index>,
    pub(crate) index_edges: usize,
    pub(crate) methods: BTreeMap<hir::PointId, dependencies::Method>,
    pub(crate) method_edges: usize,
    pub(crate) elements: BTreeMap<hir::PointId, dependencies::Element>,
    pub(crate) element_edges: usize,
    pub(crate) exclusives: BTreeMap<hir::PointId, dependencies::ExclusiveOperation>,
    pub(crate) exclusive_edges: usize,
    pub(crate) outputs: BTreeMap<hir::PointId, dependencies::Output>,
    pub(crate) output_edges: usize,
    pub(crate) unaries: BTreeMap<hir::PointId, dependencies::Unary>,
    pub(crate) unary_edges: usize,
    pub(crate) derefs: BTreeMap<hir::PointId, dependencies::Deref>,
    pub(crate) deref_edges: usize,
    pub(crate) fields: BTreeMap<hir::PointId, dependencies::Field>,
    pub(crate) field_edges: usize,
    pub(crate) typed_ops: BTreeMap<hir::PointId, dependencies::Typed>,
    pub(crate) typed_edges: usize,
    pub(crate) dispatch_ops: BTreeMap<hir::PointId, dependencies::Dispatch>,
    pub(crate) dispatch_edges: usize,
    pub(crate) reborrow_ops: BTreeMap<hir::PointId, dependencies::Reborrow>,
    pub(crate) reborrow_edges: usize,
    pub(crate) projections: BTreeMap<hir::PointId, dependencies::Projection>,
    pub(crate) projection_items: usize,
    pub(crate) projection_edges: usize,
    pub(crate) place_borrows: BTreeMap<hir::PointId, dependencies::PlaceBorrow>,
    pub(crate) place_borrow_edges: usize,
    pub(crate) temporary_borrows: BTreeMap<hir::PointId, dependencies::Temporary>,
    pub(crate) temporary_borrow_edges: usize,
    pub(crate) reborrows: usize,
    pub(crate) restarts: hir::RestartId,
    pub(crate) restart_inputs: BTreeMap<hir::RestartId, dependencies::RestartInput>,
    pub(crate) restart_queries: BTreeMap<hir::RestartId, BTreeSet<usize>>,
    pub(crate) bodies: BTreeMap<hir::BlockId, dependencies::Body>,
    pub(crate) body_facts: usize,
    pub(crate) body_inputs: BTreeMap<hir::BlockId, Vec<InputUse>>,
    pub(crate) sites: BTreeMap<hir::StatementId, dependencies::Site>,
    pub(crate) site: Option<hir::StatementId>,
    pub(crate) points: Vec<dependencies::Point>,
    pub(crate) point: Option<usize>,
    pub(crate) branch_edges: BTreeMap<hir::PointId, [dependencies::Edge; 6]>,
    pub(crate) region_edges: BTreeMap<hir::PointId, [dependencies::Edge; 2]>,
    pub(crate) sequences: BTreeMap<dependencies::SequenceSource, dependencies::Sequence>,
    pub(crate) sequence_edges: usize,
    pub(crate) scope_exits: BTreeMap<hir::PointId, dependencies::ScopeExit>,
    pub(crate) endpoints: BTreeMap<dependencies::SequenceSource, Vec<dependencies::Edge>>,
    pub(crate) endpoint_edges: usize,
    pub(crate) restart_edges: BTreeMap<hir::RestartId, dependencies::Edge>,
    pub(crate) operations: BTreeMap<hir::PointId, dependencies::Operation>,
    pub(crate) operation_edges: usize,
    pub(crate) paths: BTreeMap<hir::PointId, dependencies::PathOperation>,
    pub(crate) path_edges: usize,
    pub(crate) stores: BTreeMap<hir::PointId, dependencies::StoreOperation>,
    pub(crate) store_edges: usize,
    pub(crate) emissions: BTreeMap<hir::PointId, dependencies::Emission>,
    pub(crate) emission_sources: BTreeMap<hir::EmitId, (hir::PointId, usize)>,
    pub(crate) emission_edges: usize,
    pub(crate) statements: usize,
    pub(crate) statement: Vec<(hir::StatementId, bool)>,
    pub(crate) lengths: BTreeMap<usize, crate::list::Fact>,
    pub(crate) block_lengths: BTreeMap<usize, crate::list::Fact>,
    pub(crate) required: bool,
    pub(crate) type_work: Option<type_values::Work>,
    pub(crate) queries: Vec<queries::Query>,
    pub(crate) query_budgets: Vec<Option<required::Budget>>,
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
            queries::finish(&checker.queries, &checker.query_budgets)
                .map_err(|error| vec![error])?;
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
            derived: BTreeSet::new(),
            pointees: BTreeMap::new(),
            reference_cells: BTreeMap::new(),
            record_cells: BTreeMap::new(),
            record_shapes: BTreeMap::new(),
            record_compositions: BTreeMap::new(),
            record_pointees: BTreeMap::new(),
            control: false,
            inputs: BTreeMap::new(),
            bool_inputs: BTreeMap::new(),
            record_inputs: BTreeMap::new(),
            block: 0,
            owner: 0,
            calls: 0,
            invocations: BTreeMap::new(),
            invocation_edges: 0,
            indices: BTreeMap::new(),
            index_edges: 0,
            methods: BTreeMap::new(),
            method_edges: 0,
            elements: BTreeMap::new(),
            element_edges: 0,
            exclusives: BTreeMap::new(),
            exclusive_edges: 0,
            outputs: BTreeMap::new(),
            output_edges: 0,
            unaries: BTreeMap::new(),
            unary_edges: 0,
            derefs: BTreeMap::new(),
            deref_edges: 0,
            fields: BTreeMap::new(),
            field_edges: 0,
            typed_ops: BTreeMap::new(),
            typed_edges: 0,
            dispatch_ops: BTreeMap::new(),
            dispatch_edges: 0,
            reborrow_ops: BTreeMap::new(),
            reborrow_edges: 0,
            projections: BTreeMap::new(),
            projection_items: 0,
            projection_edges: 0,
            place_borrows: BTreeMap::new(),
            place_borrow_edges: 0,
            temporary_borrows: BTreeMap::new(),
            temporary_borrow_edges: 0,
            reborrows: 0,
            restarts: 0,
            restart_inputs: BTreeMap::new(),
            restart_queries: BTreeMap::new(),
            bodies: BTreeMap::new(),
            body_facts: 0,
            body_inputs: BTreeMap::new(),
            sites: BTreeMap::new(),
            site: None,
            points: Vec::new(),
            point: None,
            branch_edges: BTreeMap::new(),
            region_edges: BTreeMap::new(),
            sequences: BTreeMap::new(),
            sequence_edges: 0,
            scope_exits: BTreeMap::new(),
            endpoints: BTreeMap::new(),
            endpoint_edges: 0,
            restart_edges: BTreeMap::new(),
            operations: BTreeMap::new(),
            operation_edges: 0,
            paths: BTreeMap::new(),
            path_edges: 0,
            stores: BTreeMap::new(),
            store_edges: 0,
            emissions: BTreeMap::new(),
            emission_sources: BTreeMap::new(),
            emission_edges: 0,
            statements: 0,
            statement: Vec::new(),
            lengths: BTreeMap::new(),
            block_lengths: BTreeMap::new(),
            required: false,
            type_work: None,
            queries: Vec::new(),
            query_budgets: Vec::new(),
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
