use super::Primary;
use crate::check::inputs::tests::check;

#[test]
pub(crate) fn exported_inputs_retain_declaration_ids_and_private_dependencies() {
    let checker = check("private<uint8>:2;->width:private+2;->row:{->nested:{->n:width}}");
    assert_eq!(checker.module.inputs.len(), 2);
    assert!(!checker.module.inputs.contains_key("private"));
    let width = checker.module.inputs["width"].id;
    assert_eq!(checker.inputs[&width].value, Some(4));
    let row = checker.module.inputs["row"].id;
    assert_eq!(
        checker.record_inputs[&row].field(&[0, 0]).unwrap().value,
        Some(4)
    );
}

#[test]
pub(crate) fn exported_inputs_separate_initializer_effects_from_file_effects() {
    let checker =
        check("d:@\"debug\";d.print(1);->good:4;->bad:{->4;d.print(2)};->row:{->n:4;d.print(3)}");
    assert_eq!(checker.module.inputs.keys().collect::<Vec<_>>(), ["good"]);
}

#[test]
pub(crate) fn exported_inputs_exclude_mutable_conditional_and_primary_emissions() {
    for source in ["->n:=4", "|true|->n:4", "->row:{->n:=4}", "->4"] {
        assert!(check(source).module.inputs.is_empty(), "{source}");
    }
}

#[test]
pub(crate) fn exported_inputs_preserve_complete_record_work() {
    let checker = check("row:{->nested:{->n:4};unused:2};->copy:row.nested");
    let id = checker.module.inputs["copy"].id;
    let record = &checker.record_inputs[&id];
    assert_eq!(record.field(&[0]).unwrap().value, Some(4));
    assert!(record.input.work > 10);
}

#[test]
pub(crate) fn primary_inputs_preserve_checked_identity_width_and_runtime_expression() {
    use crate::hir::{ExprKind, Stmt, Type};
    let parsed = crate::parser::parse_documented("base<uint8>:2;->base+2").unwrap();
    let mut checker = crate::check::Checker::new();
    let block = checker.block(&parsed.block, None, None).unwrap();
    let (id, Primary::Int(input)) = checker.module.primary.unwrap() else {
        panic!("integer primary")
    };
    assert_eq!(input.value, Some(4));
    assert_eq!(block.stmts.len(), 2);
    assert!(
        matches!(&block.stmts[1], Stmt::Emit { id: found, field: None, value, .. }
        if *found == id && matches!(value.kind, ExprKind::Binary { .. })
            && value.ty == Type::Int { bits: 8, signed: false })
    );
}

#[test]
pub(crate) fn primary_inputs_keep_private_dependencies_tail_work_and_file_effects() {
    let checker =
        check("d:@\"debug\";d.print(1);base:2;->{->base*2;unused:3};d.print(2);->named:7");
    let (_, Primary::Int(input)) = checker.module.primary.unwrap() else {
        panic!("integer primary")
    };
    assert_eq!(input.value, Some(4));
    assert!(input.work > 6);
    assert_eq!(checker.module.inputs.len(), 1);
    assert_eq!(
        checker.inputs[&checker.module.inputs["named"].id].value,
        Some(7)
    );
}

#[test]
pub(crate) fn primary_inputs_exclude_effects_mutable_sources_conditionals_and_other_kinds() {
    for source in [
        "d:@\"debug\";->{->4;d.print(1)}",
        "value:=4;->value",
        "|true|->4",
        "|false|->4",
        "->{->n:4}",
        "->4.0",
        "f<int32>:(){->4};->f()",
    ] {
        assert!(check(source).module.primary.is_none(), "{source}");
    }
}

#[test]
pub(crate) fn composed_inputs_keep_source_ids_without_changing_runtime_emissions() {
    use crate::ast::{Expr, ExprKind};
    use crate::check::{Checker, Value};
    use crate::hir;
    let block = |source| {
        let parsed = crate::parser::parse_documented(source).unwrap();
        Expr {
            span: parsed.block.span,
            kind: ExprKind::Block(parsed.block),
        }
    };
    let mut checker = Checker::new();
    let (value, source) = checker
        .module_value(
            &block(
                "private<uint8>:2;->private;->width:private;->row:{->n:private};->label:\"ready\"",
            ),
            None,
        )
        .unwrap();
    let width = source.inputs["width"].clone();
    let row = source.inputs["row"].clone();
    let Primary::Int(primary) = source.primary.as_ref().unwrap().1.clone() else {
        panic!("integer primary")
    };
    let id = checker.local(value.ty.clone());
    checker.exports.insert(id, source);
    checker
        .declare(
            "source",
            Value::FileModule {
                id,
                ty: value.ty.clone(),
            },
            value.span,
        )
        .unwrap();
    let locals = checker.locals.len();
    let (facade, exports) = checker.module_value(&block("->source"), None).unwrap();
    assert_eq!(facade.ty, value.ty);
    assert_eq!(checker.locals.len(), locals + 1);
    assert_eq!(exports.inputs.len(), 2);
    assert_eq!(exports.inputs["width"].id, width.id);
    assert_eq!(exports.inputs["row"].id, row.id);
    assert_eq!(exports.inputs["width"].work, width.work + 2);
    assert_eq!(exports.inputs["row"].work, row.work + 2);
    assert!(!checker.inputs.contains_key(&id));
    assert!(!checker.record_inputs.contains_key(&id));
    assert!(!checker.record_inputs.contains_key(&locals));
    let (emitted, Primary::Int(input)) = exports.primary.unwrap() else {
        panic!("integer primary")
    };
    assert_eq!(input.value, primary.value);
    assert_eq!(input.work, primary.work + 2);
    let hir::ExprKind::Block(body) = facade.kind else {
        panic!("module block")
    };
    assert_eq!(body.stmts.len(), 5);
    assert!(
        matches!(&body.stmts[0], hir::Stmt::Bind { value, .. } if matches!(value.kind, hir::ExprKind::Local(source) if source == id))
    );
    assert!(
        matches!(&body.stmts[1], hir::Stmt::Emit { id, field: None, value, .. } if *id == emitted && matches!(value.kind, hir::ExprKind::Primary(_)))
    );
    assert!(body.stmts[2..].iter().all(|stmt| matches!(stmt, hir::Stmt::Emit { field: Some(_), value, .. } if matches!(value.kind, hir::ExprKind::Field { .. }))));
}

#[test]
pub(crate) fn exported_paths_retain_record_ancestor_work_and_values() {
    use crate::check::{
        exports::{Input, Module},
        inputs::Sources,
    };
    let mut checker = check("->row:{->nested:{->n:4};unused:2}");
    let id = checker.module.inputs["row"].id;
    let ty = checker.locals[id].clone();
    let root = checker.local(ty);
    let record = checker.record_inputs[&id].clone();
    let mut module = Module::default();
    module.inputs.insert(
        "nested".into(),
        Input {
            id,
            path: vec![0],
            work: 2,
        },
    );
    checker.exports.insert(root, module);
    let source = checker.input_path(root, &[0, 0]).unwrap();
    assert_eq!(source.id, id);
    assert_eq!(source.path, [0, 0]);
    assert_eq!(source.work, 3);
    let input = checker
        .field_input(root, &[0, 0], &Sources::default())
        .unwrap();
    assert_eq!(input.value, Some(4));
    assert_eq!(input.work, record.input.work + 5);
    assert!(checker.input_path(root, &vec![0; 34]).is_none());
}

#[test]
pub(crate) fn composed_record_exports_share_complete_evidence_and_keep_runtime_hir() {
    use crate::hir::{ExprKind, Stmt};
    let parsed = crate::parser::parse_documented("row:{->z:4;->a:{->n:2};unused:1};->row").unwrap();
    let mut checker = crate::check::Checker::new();
    let block = checker.block(&parsed.block, None, None).unwrap();
    let a = &checker.module.inputs["a"];
    let z = &checker.module.inputs["z"];
    assert_eq!(a.id, z.id);
    assert_eq!(a.path, [0]);
    assert_eq!(z.path, [1]);
    let record = &checker.record_inputs[&a.id];
    assert_eq!(record.field(&[0, 0]).unwrap().value, Some(2));
    assert_eq!(record.field(&[1]).unwrap().value, Some(4));
    assert!(record.input.work > 10);
    assert!(
        matches!(&block.stmts[1], Stmt::Bind { id, value } if *id == a.id && matches!(value.kind, ExprKind::Local(_)))
    );
    assert!(
        matches!(&block.stmts[2], Stmt::Emit { field: None, value, .. } if matches!(value.kind, ExprKind::Primary(_)))
    );
    assert!(block.stmts[3..].iter().all(|stmt| matches!(stmt, Stmt::Emit { field: Some(_), value, .. } if matches!(value.kind, ExprKind::Field { .. }))));
}

#[test]
pub(crate) fn composed_record_exports_bound_retained_field_paths() {
    for count in [256, 257] {
        let fields = (0..count)
            .map(|id| format!("->n{id}:1;"))
            .collect::<String>();
        let checker = check(&format!("row:{{{fields}}};->row"));
        assert_eq!(
            checker.module.inputs.len(),
            if count == 256 { 256 } else { 0 }
        );
        if count == 256 {
            let input = &checker.module.inputs["n0"];
            assert_eq!(
                checker.record_inputs[&input.id]
                    .field(&input.path)
                    .unwrap()
                    .value,
                Some(1)
            );
        }
    }
}

#[test]
pub(crate) fn boolean_exports_retain_source_ids_private_inputs_and_errors() {
    let parsed = crate::parser::parse_documented("private:true;->enabled:private").unwrap();
    let mut checker = crate::check::Checker::new();
    let block = checker.block(&parsed.block, None, None).unwrap();
    let input = &checker.module.inputs["enabled"];
    assert!(input.path.is_empty());
    assert_eq!(input.work, 0);
    assert_eq!(checker.bool_inputs[&input.id].value, Some(true));
    assert!(!checker.inputs.contains_key(&input.id));
    assert!(!checker.module.inputs.contains_key("private"));
    assert!(
        matches!(&block.stmts[1], crate::hir::Stmt::Bind { id, value } if *id == input.id && matches!(value.kind, crate::hir::ExprKind::Local(_)))
    );
    let source = "record:{->width<uint8>:255};->bad:record.width+1==0";
    let checker = check(source);
    let id = checker.module.inputs["bad"].id;
    let input = &checker.bool_inputs[&id];
    assert!(input.value.is_none());
    let error = input.error.as_ref().unwrap();
    assert_eq!(error.code, "E107");
    assert_eq!(error.span.start, source.find("record.width+1").unwrap());
}

#[test]
pub(crate) fn boolean_primaries_preserve_emission_identity_tail_work_and_errors() {
    use crate::hir::{ExprKind, Stmt, Type};
    let parsed = crate::parser::parse_documented(
        "d:@\"debug\";d.print(1);private:false;->{->private;unused:2};->named:true",
    )
    .unwrap();
    let mut checker = crate::check::Checker::new();
    let block = checker.block(&parsed.block, None, None).unwrap();
    let (id, Primary::Bool(input)) = checker.module.primary.unwrap() else {
        panic!("boolean primary")
    };
    assert_eq!(input.value, Some(false));
    assert!(input.work > 5);
    assert!(input.error.is_none());
    assert!(block.stmts.iter().any(|stmt| matches!(stmt,
        Stmt::Emit { id: found, field: None, value, .. }
        if *found == id && value.ty == Type::Bool && matches!(value.kind, ExprKind::Block(_))
    )));
    let source = "row:{->width<uint8>:255};->row.width+1==0";
    let checker = check(source);
    let (_, Primary::Bool(input)) = checker.module.primary.unwrap() else {
        panic!("boolean primary")
    };
    assert!(input.value.is_none());
    let error = input.error.unwrap();
    assert_eq!(error.code, "E107");
    assert_eq!(error.span.start, source.find("row.width+1").unwrap());
    for source in [
        "value:=true;->value",
        "|true|->true",
        "d:@\"debug\";->{->false;d.print(1)}",
        "get<boolean>:(){->true};->get()",
    ] {
        assert!(check(source).module.primary.is_none(), "{source}");
    }
}

#[test]
pub(crate) fn named_type_exports_keep_concrete_payloads_without_runtime_fields() {
    use crate::check::Value;
    use crate::hir::Type;
    let source = "c:@\"core\";<Kind>:<c.Type>;private<Kind>:<uint8>;->element<Kind>:private;->items<Type>:{count:4;-><(element)[count]>};->copy<Type>:items";
    let parsed = crate::parser::parse(source).unwrap();
    let mut checker = crate::check::Checker::new();
    let block = checker.block(&parsed, None, None).unwrap();
    assert!(block.stmts.is_empty());
    assert!(checker.locals.is_empty());
    assert!(checker.module.inputs.is_empty());
    assert!(checker.module.primary.is_none());
    assert_eq!(checker.module.values.len(), 3);
    assert!(matches!(
        checker.module.values["element"],
        Value::Type(Type::Int {
            bits: 8,
            signed: false
        })
    ));
    let Value::Type(items) = &checker.module.values["items"] else {
        panic!("type value")
    };
    let Value::Type(copy) = &checker.module.values["copy"] else {
        panic!("type value")
    };
    assert_eq!(items, copy);
    crate::compile(&format!("{source};v<(copy)>:[1,2,3,4]")).unwrap();
}

#[test]
pub(crate) fn named_type_exports_preserve_kinds_scope_and_storage_gates() {
    for (source, code) in [
        ("->kind<Type>:=<int32>", "B001"),
        ("|true|->kind<Type>:<int32>", "B001"),
        ("|false|->kind<Type>:<int32>", "B001"),
        ("row:{->kind<Type>:<int32>}", "B001"),
        ("f<int32>:(){->kind<Type>:<int32>;->1}", "B001"),
        ("->kind<Type>:7", "E207"),
        ("->kind<Type>:{->false}", "E207"),
        ("->kind<Type>:missing", "E201"),
        ("->kind<Type>:<Type>", "B001"),
        ("->make<Type>:(){-><int32>}", "B001"),
        ("->kind<Type>:<int32>;->kind<Type>:<uint8>", "E205"),
        ("->kind:7;->kind<Type>:<int32>", "E205"),
        ("->kind<Type>:<int32>;->kind:7", "E205"),
        ("kind:7;->kind<Type>:<int32>", "E203"),
        ("->kind<Type>:{local:4;-><int32>};outside:local", "E201"),
        ("->kind<Type>:<int32>;p:&kind", "B001"),
    ] {
        let error = crate::compile(source).unwrap_err().remove(0);
        assert_eq!(error.code, code, "{source}: {error:?}");
    }
    crate::compile("<Type>:<uint8>;->n<Type>:4").unwrap();
    crate::compile("->kind<Type>:<int32>;->n<(kind)>:4;->get<int32>:(){->7}").unwrap();
}

#[test]
pub(crate) fn named_type_exports_document_the_annotation_and_concrete_alias() {
    let source = "c:@\"core\";#| Element. |#->element<c.Type>:<int32>;#| Items. |#->items<Type>:{-><(element)[4]>};#| Alias. |#-><Items>:items";
    let (_, model) = crate::documentation::checked(source, true).unwrap();
    let model = model.unwrap();
    for (name, signature) in [
        ("element", "core.Type"),
        ("items", "core.Type"),
        ("Items", "int32[4]"),
    ] {
        let entry = model
            .entries
            .iter()
            .find(|entry| entry.name == name)
            .unwrap();
        assert!(entry.checked);
        assert!(entry.public);
        assert_eq!(entry.signature, signature);
    }
}
