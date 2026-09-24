mod aliases;
mod fields;
mod foundation;
mod paths;
mod restarts;

pub(crate) fn accepts(source: &str) {
    if let Err(errors) = crate::compile(source) {
        panic!("{source}\n{errors:?}");
    }
}

pub(crate) fn rejects(source: &str, code: &str) {
    let errors = crate::compile(source).expect_err(source);
    assert_eq!(errors[0].code, code, "{source}\n{errors:?}");
}

#[test]
pub(crate) fn integer_literals_respect_expected_width_and_negation_boundary() {
    accepts("x<int8>:-128;y<uint64>:18446744073709551615;z<int16>:-32_768");
    rejects("x<int8>:128", "E216");
    rejects("x<int8>:-(128)", "E216");
    rejects("x<uint8>:-1", "E222");
    rejects("x:2147483648", "E216");
}

#[test]
pub(crate) fn checked_arithmetic_propagates_immutable_constants() {
    rejects("x<int8>:127;y:x+1", "E107");
    rejects("x<int8>:-128;y:-x", "E107");
    rejects("x:1/0", "E107");
    accepts("x<int8>:=127;y:x+1");
    accepts("x:false&&(1/0==0)");
    accepts("x<int64>:-9223372036854775808;y:x%-1");
    rejects("x<int64>:-9223372036854775808;y:x/-1", "E107");
    accepts("flag:=true;value:'v {|flag|{'v->1;'v.leave()};->0};result:1/value");
}

#[test]
pub(crate) fn float32_literals_round_once_from_decimal() {
    let program = crate::compile("x<float32>:1.0000000596046447753906250000000001").unwrap();
    let crate::hir::Stmt::Bind { value, .. } = &program.body.stmts[0] else {
        panic!("binding");
    };
    let crate::hir::ExprKind::Float(value) = value.kind else {
        panic!("float");
    };
    assert_eq!(value, f64::from(f32::from_bits(0x3f800001)));
    rejects("x<float32>:3.5e38", "E216");
    accepts("x<float32>:1.0e-99");
}

#[test]
pub(crate) fn typed_numeric_values_never_widen_silently() {
    accepts("x<uint8>:2;y:1+x;z<uint8>:y");
    rejects("x<int8>:1;y<int16>:2;z:x+y", "E213");
    rejects("x<int8>:1;y<int16>:x", "E207");
}

#[test]
pub(crate) fn scalar_assignment_and_matcher_domains_are_checked() {
    rejects("x:1;x=2", "E305");
    rejects("x:=1;x=\"two\"", "E207");
    rejects("|1|x:2", "E215");
    rejects("x:true+false", "E222");
    rejects("x<float32>:1.0;y<float64>:1.0;z:x+y", "E213");
}

#[test]
pub(crate) fn lexical_aliases_preserve_intrinsic_identity() {
    accepts(
        "core:@\"core\";debug:@\"debug\";print:debug.print;{true:\"shadow\";x<core.boolean>:core.true;print(x)}",
    );
    accepts("<T>:<int8>;T:42;x<T>:1");
    rejects("x:1;x:2", "E203");
    rejects("x<missing>:1", "E202");
    rejects("x:missing", "E201");
}

#[test]
pub(crate) fn blocks_construct_records_and_project_scalar_primaries() {
    accepts(
        "reading:{->24;->unit:\"celsius\"};answer<int32>:reading+1;copy<int32>:reading;unit:reading.unit",
    );
    accepts("outer:{->{->42;->unit:\"u\"};->ready:true};unit:outer.unit");
    rejects("x:{->1;->2}", "E205");
    rejects("x:{->name:1;->name:2}", "E205");
}

#[test]
pub(crate) fn normal_paths_must_initialize_required_results() {
    accepts("f<int32>:(flag<boolean>) 'result {|flag|{'result->1;'result.leave()};->2}");
    rejects("f<int32>:(flag<boolean>){|flag|->1}", "E204");
    rejects("f<int32>:(flag<boolean>){|flag|->1;->2}", "E205");
    accepts("x:{|false|->1;->2}");
}

#[test]
pub(crate) fn independent_matchers_share_flow_facts() {
    let source = format!("flag:=true;{}", "|flag|x:1;".repeat(1000));
    accepts(&source);
}

#[test]
pub(crate) fn forward_groups_support_mutual_recursion_and_require_adjacency() {
    accepts(
        "even<(uint32)->boolean>;odd<(uint32)->boolean>;even<boolean>:(n<uint32>) 'r {|n==0|{'r->true;'r.leave()};->odd(n-1)};odd<boolean>:(n<uint32>) 'r {|n==0|{'r->false;'r.leave()};->even(n-1)};x:even(4)",
    );
    rejects("f<()->int32>;x:1;f<int32>:(){->1}", "E221");
    rejects("f<()->int32>;f<boolean>:(){->true}", "E221");
    rejects("f<()->int32>", "E221");
}

#[test]
pub(crate) fn scoped_control_aliases_obey_lexical_lifetime() {
    accepts("x:'work {finish:'work.leave;->1;finish()}");
    accepts("n:=0;'work {n=n+1;|n<3|'work.restart()}");
    rejects("'work {}; 'work.leave()", "E201");
    rejects("x:'work {f:(){'work.leave()}}", "E201");
}

#[test]
pub(crate) fn unsupported_features_have_capability_diagnostics() {
    rejects("x<int32[]>:[]", "B001");
    rejects("x:1;f<int32>:(){->x}", "B001");
    rejects("x:1;text:\"{x}\"", "B001");
    rejects("f<int32>:(){->1};x:f==f", "E222");
}

#[test]
pub(crate) fn shared_references_keep_storage_types_and_copy_values() {
    accepts("a:1;r<&int32>:&a;s:r;v:*s;same:r==s");
    accepts("a:{->x:1;->nested:{->y:true}};r:&a;v:r.x;s:&(a.nested.y);t:*s");
    accepts("f<int32>:(){a:9;r:&a;->*r};v:f()");
    accepts("x<int32><null>:1;r:&x;v:*r");
    rejects("x:1;v:*x", "E222");
    rejects("x:1;r<&int64>:&x", "E207");
}

#[test]
pub(crate) fn record_equality_preserves_aggregate_and_scalar_contexts() {
    accepts("a:1;b:2;left:{->view:&a};right:{->view:&b};same:left=={->right}");
    accepts("a:1;b:2;left:{->view:&a};same:left=={->view:&b}");
    accepts("a:1;n<int64>:7;packet:{->n;->view:&a};same:packet==7;reverse:7==packet");
    accepts("a:1;n<uint8>:7;packet:{->n;->view:&a};same:packet==7");
    accepts("a:=1;n<int64>:7;packet:{->n;->view:&a};a=2;same:packet=={->7};reverse:{->7}==packet");
    accepts(
        "f<boolean>:(flag<boolean>){a:1;n<int64>:7;packet:{->n;->view:&a};->packet=='result {|flag|{'result->7;'result.leave()};->7}}",
    );
    rejects(
        "a:1;n<uint8>:7;packet:{->n;->view:&a};same:packet==256",
        "E216",
    );
}

#[test]
pub(crate) fn union_record_constructors_keep_member_context_and_defaults() {
    accepts(
        "<R>:<{view<&int32><null>}>;a:1;u<R><null>:{->view:&a};|u<R>|{|u.view<&int32>|x:*(u.view<&int32>)}",
    );
    accepts("<R>:<{view<&int32><null>;count<int64>}>;u<R><null>:{->count:7};|u<R>|x:u.count");
    accepts("<R>:<{-><int64>;tag<string>}>;u<R><null>:{->7;->tag:\"ok\"};|u<R>|x<int64>:u");
    accepts("<R>:<{value<uint8>;view<&int32>}>;a:1;u<R><null>:{->value:255;->view:&a}");
    rejects("<R>:<{value<uint8>}>;u<R><null>:{->value:256}", "E216");
    rejects(
        "<A>:<{value<int32>}>;<B>:<{value<int64>}>;u<A><B>:{->value:7}",
        "E207",
    );
    rejects(
        "<A>:<{value<int32>}>;<B>:<{value<int32><null>}>;u<A><B>:{->value:7}",
        "E207",
    );
    rejects(
        "<A>:<{view<&int32>}>;<B>:<{view<&string>}>;s:\"x\";u<A><B>:{->view<&int32>:&s}",
        "E207",
    );
}

#[test]
pub(crate) fn union_equality_requires_the_same_normalized_union_type() {
    accepts("a:1;u<&int32><null>:&a;empty<&int32><null>:null;same:u==empty");
    rejects("a:1;u<&int32><null>:&a;same:u==null", "E222");
    rejects("a:1;u<&int32><null>:&a;same:null==u", "E222");
    rejects("a:1;u<&int32><null>:&a;same:u==&a", "E222");
    rejects("u<int8><null>:7;same:u==7", "E222");
    rejects("u<int8><null>:7;value<int32>:7;same:u==value", "E222");
}

#[test]
pub(crate) fn reference_capability_boundaries_are_explicit() {
    accepts("x:=1;r:&!x");
    accepts("f<int32>:(x<&!int32>){->*x}");
    rejects("f<int32>:(x<int32>){r:&!x;->*r}", "E305");
    rejects("x:1;r:&x;s:&!*r", "E305");
    for source in [
        "x:1;r:&x;s:&r;v:**s",
        "x:1;r:=&x",
        "r:&(1+2)",
        "<R>:<{value<&int32>}>;f<null>:(x<&R>){->null}",
        "x:1;r:&x;r.{v:&$}",
    ] {
        accepts(source);
    }
    for source in [
        "x:1;r:&x;debug:@\"debug\";debug.print(r)",
        "<R>:<{x<int32>}>;record<R>:{->x:1};x<R><null>:record;|x<R>|{r:&(x.x)}",
    ] {
        rejects(source, "B001");
    }
}

#[test]
pub(crate) fn union_members_keep_numeric_context_without_widening() {
    accepts("x<int8><null>:-128;y<float32><null>:1.5");
    accepts("x<{count<int8><null>}>:{->count<int8>:127}");
    accepts("<T>:<string><null><never><string>;x<T>:null;y<null><string>:x");
    rejects("x<int8><null>:128", "E216");
    rejects("x<int8>:1;y<int16><null>:x", "E207");
    rejects("x<int8><int32>:1", "E207");
    rejects("x<float32><float64>:1.5", "E207");
    rejects("x<string><null>:null;y<string>:x", "E207");
    rejects("x<string><null>:null;y:x<string>", "E208");
}

#[test]
pub(crate) fn conditional_slots_join_only_completing_paths() {
    accepts("make:(flag<boolean>){|flag|->name:\"hello\"};value<{name<string><null>}>:make(false)");
    accepts("f<int32><null>:(flag<boolean>){|flag|->1}");
    accepts("x<{name<string><null>}>:{}");
    accepts("x<{name<string><null>}>:{->{->name:\"ok\"}}");
    accepts("x<{count<int8>;name<string><null>}>:{->{->count:127};->name:\"ok\"}");
    rejects(
        "f<{name<string>}>:(flag<boolean>){|flag|->name:\"x\"}",
        "E204",
    );
    rejects("x<{count<int8>}>:{->{->count:128}}", "E216");
    rejects("x<{name<string><null>}>:{->{->other:1}}", "E207");
    let program =
        crate::compile("n:=0;x:'loop {n=n+1;|n<2|{'loop->gone:1;'loop.restart()}}").unwrap();
    assert_eq!(program.locals.last(), Some(&crate::hir::Type::Null));
}

#[test]
pub(crate) fn stable_predicates_prove_disjoint_emissions() {
    accepts("f<int32>:(v<string><null>){|v<null>|->1;|!(v<null>)|->2}");
    accepts("f<int32>:(v<string><int32><null>){|v<null>|->1;|v<string>|->2;|v<int32>|->3}");
    accepts("f<int32>:(flag<boolean>){|flag|->1;|!flag|->2}");
    rejects(
        "f<int32>:(v<string><null>){|v<null>|->1;|v<null>|->2}",
        "E205",
    );
    rejects("f<int32>:(v<int32>){|v>0|->1;|v>10|->2}", "E205");
    accepts("f<int32>:(v<string><int32><null>){|!(v<int32>)&&v<string>|->1;|v<int32><null>|->2}");
}

#[test]
pub(crate) fn distinct_record_variants_keep_their_field_tag_domains() {
    accepts(
        "<A>:<{name<string><null>}>;<B>:<{name<int32><null>}>;f<null>:(x<A><B>){|x<A>|{|x.name<string>|y:x.name<string>};|x<B>|{|x.name<int32>|y:x.name+1}}",
    );
}

#[test]
pub(crate) fn narrowing_tracks_short_circuit_and_leaving_paths() {
    accepts("f<string>:(v<string><null>) 'r {|v<null>|{'r->\"fallback\";'r.leave()};->v<string>}");
    accepts("f<null>:(v<string><null>){|v<string>&&v.size()>0|x:v<string>}");
    accepts("f<null>:(v<string><null>){|v<null>||v.size()==0|{}}");
    accepts("f<null>:(v<{name<string><null>}>){|!(v.name<null>)|x:v.name<string>}");
    rejects("f<null>:(v<string><null>){|v<null>|{};x:v<string>}", "E208");
    rejects(
        "f<null>:(v<string><null>){|v<string>||true|x:v<string>}",
        "E208",
    );
}

#[test]
pub(crate) fn assignment_invalidates_scalar_and_field_proofs() {
    rejects(
        "v<string><null>:=\"x\";|v<string>|{v=null;x:v<string>}",
        "E208",
    );
    rejects(
        "v<{name<string><null>}>:={->name:\"x\"};|v.name<string>|{v={};x:v.name<string>}",
        "E208",
    );
    rejects(
        "v<string><null>:=\"x\";|v<null>|v=\"x\";x:v<string>",
        "E208",
    );
    accepts("v<string><null>:=\"x\";|v<string>|{x:v<string>;v=null}");
}

#[test]
pub(crate) fn restart_drops_mutable_proofs_and_rejects_outer_slot_hazards() {
    rejects(
        "v<string><null>:=\"x\";|v<string>|'loop {x:v<string>;v=null;'loop.restart()}",
        "E208",
    );
    rejects("x:'outer {'inner {'outer->1;'inner.restart()}}", "B001");
    accepts("n:=0;x:'loop {n=n+1;|n<2|{'loop->1;'loop.restart()};->2}");
}
#[test]
pub(crate) fn parameter_and_receiver_addresses_are_scope_local() {
    accepts("read<int32>:(value<int32>){view:{->&value};->*view}");
    accepts("value:7;copy:value.{view:&$;->*view}");
    rejects("bad<&int32>:(value<int32>){->&value}", "E303");
    rejects("bad:(value<int32>){->(&value).{->&*$}}", "E303");
    rejects("value:7;view:value.{->&$}", "E303");
    rejects("value:{->n:7};view:value.{->&($.n)}", "E303");
}

#[test]
pub(crate) fn shared_dispatch_keeps_original_origins_and_bounds() {
    accepts(
        "<R>:<{n<int32>}>;<H>:<{view<&R>}>;field<&int32>:(holder<H>){->&(holder.view.n)};owner<R>:{->n:7};view:field({->view:&owner});read:*view",
    );
    accepts("owner:=1;view:(&owner).{->&*$};value:*view;owner=2");
    accepts("a:=1;b:=2;pair:{->a:&a;->b:&b};view:pair.{->$.a};b=3;value:*view");
    rejects("owner:=1;view:(&owner).{->$};owner=2;value:*view", "E302");
    rejects(
        r#"first<&int32>:(a<&int32>,b<&string>){->a};owner:1;view:{short:"x";->first(&owner,&short).{->$}}"#,
        "E303",
    );
}

#[test]
pub(crate) fn unary_context_does_not_inject_a_union_before_the_operator() {
    accepts("value<boolean><null>:!true");
    accepts("byte<int8>:5;value<int8><null>:-byte;bits<int8><null>:~byte");
    accepts("value<float32><null>:-({->1.5})");
    rejects("value:!1", "E222");
    rejects("value<int8><null>:-(128)", "E216");
    rejects("value<uint8><null>:-(1)", "E222");
    rejects("byte<int8>:5;value<int16>:-byte", "E207");
    rejects("value<float32><float64>:-(1.5)", "E207");
}

#[test]
pub(crate) fn dispatch_receiver_sigil_has_scoped_immutable_identity() {
    accepts("value:3;out:value.{copy:$;->{->$+copy}}");
    accepts("value:=3;out:(&value).{->*$}");
    rejects("out:$", "E201");
    rejects("value:3;out:value.{$=4;->$}", "E305");
    rejects("$:3", "E004");
}
