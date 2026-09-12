use super::*;

pub(crate) fn check(source: &str) -> Checker {
    let parsed = crate::parser::parse_documented(source).unwrap();
    let mut checker = Checker::new();
    checker.block(&parsed.block, None, None).unwrap();
    checker
}

#[test]
pub(crate) fn initializer_inputs_track_immutable_integer_dependencies() {
    let checker = check("base<uint8>:2;next:base+2;alias:next;mutable:=4");
    assert_eq!(checker.inputs.len(), 3);
    assert!(checker.inputs.values().all(|input| input.error.is_none()));
    let costs = checker
        .inputs
        .values()
        .map(|input| input.work)
        .collect::<Vec<_>>();
    assert!(costs[1] > costs[0]);
    assert!(costs[2] > costs[1]);
}

#[test]
pub(crate) fn initializer_inputs_exclude_folded_blocks_calls_and_mutable_reads() {
    for source in [
        "d:@\"debug\";x:{d.print(1);->4};alias:x",
        "x:=4;copy:x;alias:copy",
        "f<int32>:(x<int32>){copy:x;->copy};result:f(4)",
        "d:@\"debug\";row:{->n:4;d.print(1)};copy:row.n",
    ] {
        assert!(check(source).inputs.is_empty(), "{source}");
    }
}

#[test]
pub(crate) fn initializer_inputs_retain_unreachable_arithmetic_failures() {
    let checker = check("|false|{base<uint8>:255;bad:base+1;alias:bad}");
    let errors = checker
        .inputs
        .values()
        .filter_map(|input| input.error.as_ref())
        .collect::<Vec<_>>();
    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(|error| error.code == "E107"));
    assert_eq!(errors[0].span, errors[1].span);
}

#[test]
pub(crate) fn initializer_inputs_retain_values_after_lexical_scopes_close() {
    let checker = check("base<uint8>:252;complement:~base;wide<uint64>:4294967296;next:wide+1");
    assert_eq!(
        checker
            .inputs
            .values()
            .map(|input| input.value)
            .collect::<Vec<_>>(),
        [Some(252), Some(3), Some(4294967296), Some(4294967297)]
    );
}

#[test]
pub(crate) fn initializer_inputs_keep_first_invalid_child_before_parent_errors() {
    let source = "|false|{bad<uint8>:(255+1)+(254+2);alias:bad}";
    let checker = check(source);
    let start = source.find("255+1").unwrap();
    for input in checker.inputs.values() {
        assert!(input.value.is_none());
        assert_eq!(input.error.as_ref().unwrap().span.start, start);
    }
}

#[test]
pub(crate) fn initializer_blocks_preserve_local_values_nested_blocks_and_post_emission_work() {
    let checker = check("base:2;capacity:{step:base+1;->step*2;unused:7};copy:capacity");
    let input = checker.inputs.values().last().unwrap();
    assert_eq!(input.value, Some(6));
    assert!(input.error.is_none());
    for source in [
        "capacity:{step:2;->step*2};<T>:{n:capacity;-><int32[n]>};v<T>:[1,2]",
        "capacity:{step:{->2};->step*2};<T>:{-><int32[capacity]>}",
        "capacity<uint8>:{step<uint8>:252;->~step};<T>:{-><int32[capacity]>}",
        "capacity:{->4;unused:3};<T>:{-><int32[capacity]>}",
    ] {
        crate::compile(source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
    }
}

#[test]
pub(crate) fn initializer_blocks_reject_every_effect_or_mutation_even_after_emission() {
    for source in [
        "d:@\"debug\";capacity:{->4;d.print(1)}",
        "capacity:{scratch:=1;->4}",
        "capacity:{scratch:=1;scratch=2;->4}",
        "capacity:{|true|->4}",
        "f<int32>:(){->4};capacity:{->f()}",
    ] {
        let checker = check(source);
        assert!(checker.inputs.is_empty(), "{source}");
        assert!(crate::compile(&format!("{source};<T>:{{n:capacity;-><int32[n]>}}")).is_err());
    }
}

#[test]
pub(crate) fn initializer_blocks_keep_unused_integer_failures_after_the_primary() {
    let source = "|false|{capacity<uint8>:{->4;unused<uint8>:255+1};<T>:{n:capacity;-><int32>}}";
    let error = crate::compile(source).unwrap_err().remove(0);
    assert_eq!(error.code, "E107", "{error:?}");
    assert_eq!(error.span.start, source.find("255+1").unwrap());
}

#[test]
pub(crate) fn initializer_scopes_preserve_shadowing_and_nested_values() {
    let checker = check("base:9;capacity:{base:2;step:{base:base+1;->base};->base*step}");
    assert_eq!(checker.inputs.values().last().unwrap().value, Some(6));
    crate::compile("base:9;capacity:{base:2;step:{base:base+1;->base};->base*step};<T>:{n:capacity;-><int32[n]>}").unwrap();
}

#[test]
pub(crate) fn initializer_scopes_keep_nested_unused_failures_in_source_order() {
    let source = "|false|{capacity<uint8>:{base<uint8>:255;step<uint8>:{unused<uint8>:base+1;->2};->4};<T>:{n:capacity;-><int32>}}";
    let error = crate::compile(source).unwrap_err().remove(0);
    assert_eq!(error.code, "E107");
    assert_eq!(error.span.start, source.find("base+1").unwrap());
}

#[test]
pub(crate) fn initializer_blocks_keep_first_failure_before_unavailable_tail() {
    for body in [
        "bad<uint8>:255+1;->4;d.print(1)",
        "->4;bad<uint8>:255+1;d.print(1)",
        "bad<uint8>:255+1;unused:get();->4",
        "bad<uint8>:255+1",
    ] {
        let source = format!(
            "d:@\"debug\";get<uint8>:(){{->4}};|false|{{n<uint8>:{{{body}}};<T>:{{v:n;-><int32>}}}}"
        );
        let error = crate::compile(&source).unwrap_err().remove(0);
        assert_eq!(error.code, "E107", "{source}: {error:?}");
        assert_eq!(error.span.start, source.find("255+1").unwrap());
    }
}

#[test]
pub(crate) fn initializer_blocks_keep_declared_noninteger_capture_and_scratch_gates() {
    for (ty, value) in [
        ("boolean", "true"),
        ("string", "\"x\""),
        ("{n<int32>}", "{->n:1}"),
    ] {
        let source = format!("|false|{{v<{ty}>:{{bad<uint8>:255+1;->{value}}}}}");
        let checker = check(&source);
        assert!(
            checker
                .inputs
                .keys()
                .all(|id| matches!(checker.locals[*id], Type::Int { .. } | Type::Never)),
            "{source}"
        );
    }
    for source in [
        "|false|{n<uint8>:{unused<boolean>:{bad<uint8>:255+1;->true};->4};<T>:{v:n;-><int32>}}",
        "|false|{flag<boolean>:{unused<string>:{bad<uint8>:255+1;->\"x\"};->true};row<{n<uint8>}>:{|flag|->n<uint8>:4};<T>:{v:row.n;-><int32>}}",
        "|false|{row<{n<uint8>}>:{unused<string>:{bad<uint8>:255+1;->\"x\"};->n<uint8>:4};<T>:{v:row.n;-><int32>}}",
    ] {
        let error = crate::compile(source).unwrap_err().remove(0);
        assert_eq!(error.code, "E211", "{source}: {error:?}");
    }
}
