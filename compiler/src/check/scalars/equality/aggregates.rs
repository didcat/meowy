use super::*;

#[test]
pub(crate) fn list_record_equality_requires_full_shapes_through_every_wrapper() {
    let prefix = "<R>:<{-><int32[2]>;tag<boolean>}>;v<int32[2]>:[1];r<R>:{->[1];->tag:true}";
    for op in ["==", "!="] {
        for body in [
            "r",
            "((r))",
            "r~<R>",
            "((r~<R>))",
            "{->r}",
            "{->((r))}",
            "r.{->$}",
            "((r.{->(($))}))",
            "{->[1];->tag:false}",
            "(({->[1];->tag:false}))",
            "0.{->[1];->tag:false}",
            "((0.{->[1];->tag:false}))",
        ] {
            rejects(&format!("{prefix};x:v {op} {body}"), "E222");
            rejects(&format!("{prefix};x:{body} {op} v"), "E222");
        }
    }
}

#[test]
pub(crate) fn list_record_equality_keeps_context_errors_and_stopped_plans() {
    rejects("v<uint8[1]>:[1];x:v=={->[256];->tag:true}", "E216");
    rejects("v<uint8[1]>:[1];x:v=={->[1,2];->tag:true}", "E103");
    rejects(
        "v:[1];r:{->[1];->tag:true};x:v=={->((r));->tag:false}",
        "E205",
    );
    rejects(
        "d:@\"debug\";v:[1];x:v=={d.panic(\"stop\");->tag:missing}",
        "E201",
    );
    for source in [
        "f<never>:(r<{-><never>;tag<boolean>}>){->r==[1]}",
        "f<never>:(r<{-><never>;tag<boolean>}>){->[1]==((r))}",
        "d:@\"debug\";v:[1];x:v==((0.{d.panic(\"stop\");->[1];->tag:true}))",
        "d:@\"debug\";v:[1];x:(({d.panic(\"stop\");->[1];->tag:true}))==v",
    ] {
        accepts(source);
    }
}
