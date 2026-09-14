use super::Case;

#[test]
pub(crate) fn type_equality_selects_normalized_types_in_both_profiles() {
    Case::new("d:@\"debug\";c:@\"core\";m:@\"memory\";kind<Type>:<int32>;<T>:{same:kind==c.int32;record:<{a<int32>;b<boolean>}> == <{b<boolean>;a<int32>}>;union:<int32><null><never> == <null><int32>;nominal:m.AllocationFailure==<m.AllocationFailure>;other:m.Allocator!=m.AllocationFailure;|same&&record&&union&&nominal&&other|-><int32[4]>};v<T>:[7];d.print(v[1])").runs(b"7\n");
    Case::new("d:@\"debug\";f<int32>:(arg<uint8>){<T>:{same:arg<> == <uint8>;other:<({n:2;-><int32[n]>})> == <int32[2]>;skip:true||(<int32[1/0]> == <Missing>);|same&&other&&skip|-><int32>};v<T>:9;->v};d.print(f(3))").runs(b"9\n");
}

#[test]
pub(crate) fn type_equality_leaves_runtime_values_and_type_helpers_gated() {
    for source in [
        "flag:<int32> == <int32>",
        "kind<Type>:<int32>;flag:kind==kind",
        "make<Type>:(){-><int32>};<T>:{flag:make()==<int32>;-><int32>}",
    ] {
        let output = Case::new(source).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"B001\""));
    }
}
