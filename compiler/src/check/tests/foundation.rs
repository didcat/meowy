use super::{accepts, rejects};

#[test]
pub(crate) fn foundation_aliases_keep_module_item_and_type_identity() {
    accepts(
        r#"m:@"memory";s:@"strings";alias:m;heap:alias.heap;copy:s.copy;again:copy;<A>:<alias.Allocator>;<B>:<A>;<E>:<m.AllocationFailure>;<S>:<s.Owned>;kind:s.Owned;<T>:<(kind)>"#,
    );
    let source = r#"m:@"memory";s:@"strings";copy:s.copy;again:copy;again("text",m.heap)"#;
    let errors = crate::compile(source).unwrap_err();
    assert_eq!(errors[0].code, "B001");
    assert!(errors[0].message.contains("strings.copy"));
}

#[test]
pub(crate) fn foundation_storage_and_construction_stay_gated() {
    for source in [
        r#"m:@"memory";s:@"strings";s.copy("text",m.heap)"#,
        r#"s:@"strings";<S>:<s.Owned>;f<S>:(){->null}"#,
        r#"s:@"strings";<S>:<s.Owned>;x<&S>:null"#,
        r#"s:@"strings";x:null;found:x~<s.Owned>"#,
        r#"s:@"strings";copy:=s.copy"#,
    ] {
        rejects(source, "B001");
    }
}

#[test]
pub(crate) fn module_shadowing_does_not_forge_foundational_identities() {
    accepts(r#"m:@"memory";alias:m;{m:{->heap:7};value:m.heap};heap:alias.heap"#);
    accepts(r#"strings:@"debug";strings.print("ordinary module alias")"#);
    rejects(r#"s:@"strings";s.missing"#, "B001");
    rejects(r#"m:@"memory";<Bad>:<m.Missing>"#, "B001");
    rejects(r#"m:{->Allocator:1};<Bad>:<m.Allocator>"#, "E202");
    rejects(r#"s:{->copy:1};s.copy("text")"#, "B001");
    rejects(r#"m:@"memory";{m:@"debug";<Bad>:<m.Allocator>}"#, "E202");
    rejects(r#"s:@"unavailable""#, "B001");
}

#[test]
pub(crate) fn nominal_copy_values_allow_storage_without_resource_construction() {
    for source in [
        r#"m:@"memory";heap<m.Allocator>:m.heap;copy:heap;mutable:=copy;mutable=m.heap"#,
        r#"m:@"memory";<E>:<m.AllocationFailure>;f<E>:(e<E>){->e}"#,
        r#"m:@"memory";f<m.Allocator>:(a<m.Allocator>){->a};result:f(m.heap)"#,
        r#"m:@"memory";f<m.AllocationFailure><null>:(e<m.AllocationFailure>){->e}"#,
        r#"m:@"memory";values<m.AllocationFailure[0]>:[]"#,
        r#"m:@"memory";a:m.heap;b:a;view:&a;copy:*view"#,
    ] {
        accepts(source);
    }
}

#[test]
pub(crate) fn nominal_values_do_not_gain_record_construction_equality_or_fields() {
    for source in [
        r#"m:@"memory";a<m.Allocator>:{->value:1}"#,
        r#"m:@"memory";e<m.AllocationFailure>:{->cause:1;->bytes:5;->alignment:1}"#,
        r#"m:@"memory";f<m.Allocator>:(e<m.AllocationFailure>){->e}"#,
    ] {
        rejects(source, "E207");
    }
    for source in [
        r#"m:@"memory";a:m.heap;b:m.heap;same:a==b"#,
        r#"m:@"memory";f<boolean>:(a<m.AllocationFailure>,b<m.AllocationFailure>){->a==b}"#,
        r#"m:@"memory";a:{->slot:m.heap};same:a==a"#,
        r#"m:@"memory";a:[m.heap];same:a==a"#,
        r#"m:@"memory";e<m.AllocationFailure><null>:null;same:e==e"#,
        r#"m:@"memory";e<m.AllocationFailure[0]>:[];same:e==e"#,
    ] {
        rejects(source, "E222");
    }
    accepts(r#"m:@"memory";r:{->slot:m.heap;->5};same:r==5"#);
    rejects(
        r#"m:@"memory";f:(e<m.AllocationFailure>){->e.bytes}"#,
        "E201",
    );
    rejects(r#"m:@"memory";d:@"debug";d.print(m.heap)"#, "B001");
    rejects(
        r#"m:@"memory";d:@"debug";f:(e<m.AllocationFailure>){d.print(e)}"#,
        "B001",
    );
}

#[test]
pub(crate) fn allocator_cell_borrows_keep_local_lifetimes_and_conflicts() {
    rejects(r#"m:@"memory";f<&m.Allocator>:(){a:m.heap;->&a}"#, "E303");
    rejects(
        r#"m:@"memory";a:=m.heap;view:&a;a=m.heap;copy:*view"#,
        "E302",
    );
    accepts(r#"m:@"memory";a:=m.heap;view:&a;copy:*view;a=m.heap"#);
}

#[test]
pub(crate) fn allocator_result_signatures_use_lifetime_analysis() {
    for source in [
        r#"m:@"memory";f<m.Allocator>:(view<&int32>){->m.heap}"#,
        r#"m:@"memory";f<m.Allocator>:(cell<&m.Allocator>){->*cell}"#,
        r#"m:@"memory";f: (view<&int32>){->slot:m.heap}"#,
    ] {
        accepts(source);
    }
}

#[test]
pub(crate) fn proof_revision_retains_uint32_identity_and_static_aliases() {
    accepts(
        r#"p:@"proof";alias:p;r:alias.revision;again:r;<T>:{n<uint32>:again;-><uint8[n]>};x<T>:[7];f<uint32>:(){->again};v<uint32>:p.revision;w:=p.revision;w=2"#,
    );
    accepts(r#"p:@"proof";r:p.revision;<T>:{same:r<> == <uint32>;|same|-><uint8>};v<T>:7"#);
    rejects(r#"p:@"proof";v<int32>:p.revision"#, "E207");
    rejects(r#"p:@"proof";v:p.revision+4294967296"#, "E216");
    accepts(
        r#"p:@"proof";r:p.revision;{p:{->revision:7};v:p.revision};proof:@"debug";proof.print(r)"#,
    );
}

#[test]
pub(crate) fn proof_queries_and_descriptor_construction_remain_unsupported() {
    for source in [
        r#"p:@"proof";p.can_copy<uint32>()"#,
        r#"p:@"proof";alias:p;query:alias.is"#,
        r#"p:@"proof";kind<Type>:<p.Result>"#,
        r#"p:@"proof";p.assert(true)"#,
    ] {
        rejects(source, "B001");
    }
}

#[test]
pub(crate) fn proof_revision_supports_direct_required_reads_and_skipped_checks() {
    accepts(
        r#"p:@"proof";<T>:{n:p.revision;ok:p.revision==1;kind:p.revision<> == <uint32>;|ok&&kind|-><uint8[n+p.revision]>};x<T>:[7,8]"#,
    );
    accepts(
        r#"p:@"proof";alias:p;f:(){<T>:{n:(alias).revision;-><uint8[n]>};x<T>:[7]};<U>:{n:(@"proof").revision;-><uint8[n]>}"#,
    );
    accepts(r#"p:@"proof";<T>:{ok:true||(p.revision/0==1);|ok|-><uint8>};x<T>:7"#);
    accepts(
        r#"p:{->revision<uint32>:2};<T>:{ok:p.revision==2;|ok|-><uint8[p.revision]>};x<T>:[7,8]"#,
    );
    rejects(r#"p:@"proof";<T>:{n:p.revision/0;-><uint8[n]>}"#, "E107");
    rejects(r#"p:@"proof";<T>:{n<int32>:p.revision;-><uint8>}"#, "E207");
    rejects(
        r#"p:@"proof";<T>:{ok:true||(p.revision==4294967296);-><uint8>}"#,
        "E216",
    );
    rejects(
        r#"p:@"proof";<T>:{ok:true||(missing.revision==1);-><uint8>}"#,
        "E201",
    );
    rejects(
        r#"p:@"proof";<T>:{ok:true||(p.missing==1);-><uint8>}"#,
        "B001",
    );
}

#[test]
pub(crate) fn proof_descriptor_type_aliases_preserve_nominal_identity_without_storage() {
    for name in ["Always", "Never", "Indeterminable", "Result", "Flags"] {
        let source = format!(r#"p:@"proof";alias:p;<A>:<alias.{name}>;<B>:<A>;-><Export>:<B>"#);
        let program = crate::compile(&source).unwrap();
        assert!(program.body.stmts.is_empty());
        assert!(program.locals.is_empty());
        let errors = crate::compile(&format!("{source};value<B>:null")).unwrap_err();
        assert_eq!(errors[0].code, "E223");
        assert!(errors[0].message.contains(&format!("proof.{name}")));
    }
    accepts(
        r#"p:@"proof";<A>:<p.Always>;{<Always>:<int32>;v<Always>:7};<Always>:<boolean>;v<Always>:true"#,
    );
    accepts(r#"p:@"proof";<T>:{<A>:<p.Always>;<B>:<A>;-><uint8>};v<T>:7"#);
    rejects(
        r#"p:@"proof";<T>:{<A>:<p.Always>;-><uint8>};<B>:<A>"#,
        "E202",
    );
    rejects(r#"p:@"debug";<A>:<p.Always>"#, "E202");
    rejects(r#"p:{->Always:1};<A>:<p.Always>"#, "E202");
}

#[test]
pub(crate) fn proof_descriptors_reject_runtime_shapes_and_keep_evaluator_gates() {
    for source in [
        r#"p:@"proof";v<p.Result>:{->always:true;->never:false;->indeterminable:false}"#,
        r#"p:@"proof";<R>:<{value<p.Flags>}>"#,
        r#"p:@"proof";<R>:<p.Result[0]>"#,
        r#"p:@"proof";<R>:<&p.Always>"#,
        r#"p:@"proof";<R>:<&!p.Never>"#,
        r#"p:@"proof";->v<p.Result>:null"#,
    ] {
        rejects(source, "E223");
    }
    for source in [
        r#"p:@"proof";f<p.Result>:(){->null}"#,
        r#"p:@"proof";f:(x<p.Always>){->null}"#,
        r#"p:@"proof";<R>:<p.Always><p.Never><p.Indeterminable>"#,
        r#"p:@"proof";<A>:<p.Always>;kind<Type>:<A>"#,
        r#"p:@"proof";kind:p.Result"#,
    ] {
        rejects(source, "B001");
    }
}
