use super::Case;

#[test]
pub(crate) fn block_type_equality_selects_types_in_both_profiles() {
    Case::new("d:@\"debug\";kind<Type>:<int32>;<T>:{same:({n<uint8>:2;-><int32[n]>})==({-><(kind)[2]>});other:({-><uint8>})!=<int32>;|same&&other|-><int32[2]>};v<T>:[3,7];d.print(v[2])").runs(b"7\n");
    Case::new("d:@\"debug\";f<int32>:(arg<uint8>){<T>:{same:({local:arg<>;->local})==<uint8>;other:<int32> == ({|same|->{-><int32>};|!same|-><boolean>});|other|-><int32>};v<T>:9;->v};d.print(f(3))").runs(b"9\n");
}
