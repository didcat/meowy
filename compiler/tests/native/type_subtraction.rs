use super::Case;

#[test]
pub(crate) fn type_subtraction_executes_constructed_types_and_static_queries() {
    Case::new("d:@\"debug\";<Maybe>:<int32><null>;kind<Type>:<Maybe>!<null>;<Items>:{same:kind==<int32>;|same|-><(kind)[2]>};v<Items>:[3,7];d.print(v[2]);f<int32>:(arg<uint8>){<T>:arg<>!<null>;local<T>:arg;->7};d.print(f(2))").runs(b"7\n7\n");
    Case::new("d:@\"debug\";m:@\"memory\";kind<Type>:<m.AllocationFailure><null>!<null>;<T>:{same:kind==m.AllocationFailure;empty:kind!<m.AllocationFailure> == <never>;|same&&empty|-><int32>};v<T>:9;d.print(v)").runs(b"9\n");
}
