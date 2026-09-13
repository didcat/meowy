use super::file_modules::case;

#[test]
pub(crate) fn required_comparisons_support_arithmetic_fields_primaries_and_functions() {
    case(
        "m:@\"./facade.mwy\";<T>:{a:m.width+1==5;b:1+2<m;c:~m.width>=250;flag:(a&&b)||c;->flag<>};v<T>:false;f<boolean>:(){p:@\"./facade.mwy\";<U>:{a:p>0;b:p.row.n<=4;c:p+0==4;->c<>};v<U>:true;->v};d:@\"debug\";d.print(v);d.print(f());d.print(m.label)",
        &[("data.mwy", "base<uint8>:4;->base;->width:base;->row:{->n:base};->label:\"ready\""), ("facade.mwy", "m:@\"./data.mwy\";->m")],
    ).runs(b"false\ntrue\nready\n");
    case("value<int8>:-128;<T>:{a:-128==value;b:value<=127;c:(value+1)>value;->c<>};v<T>:true;d:@\"debug\";d.print(v)", &[]).runs(b"true\n");
}

#[test]
pub(crate) fn required_comparisons_check_skipped_widths_and_keep_evidence_gates() {
    for (body, code) in [
        ("<T>:{flag:m.width==wide;->flag<>}", "E213"),
        ("<T>:{flag:false&&(m.width==256);->flag<>}", "E216"),
        ("<T>:{flag:true||(256<m.width);->flag<>}", "E216"),
        ("<T>:{flag:-1<m.width;->flag<>}", "E222"),
        ("<T>:{flag:m.width+252==0;->flag<>}", "E107"),
        ("<T>:{flag:m.bad==0;->flag<>}", "E211"),
        ("<T>:{flag:false&&(missing==0);->flag<>}", "E201"),
        ("<T>:{flag:m==m;->flag<>}", "B001"),
        ("<T>:{flag:1.0<2.0;->flag<>}", "B001"),
        ("f<uint8>:(){<T>:{flag:m>0;->flag<>};->m+0}", "B001"),
    ] {
        let source = format!("m:@\"./data.mwy\";wide<uint16>:4;{body}");
        let output = case(
            &source,
            &[(
                "data.mwy",
                "d:@\"debug\";base<uint8>:4;->base;->width:base;->bad:{d.print(9);->1}",
            )],
        )
        .command("run", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{body}");
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(&format!("\"code\":\"{code}\"")),
            "{body}: {error}"
        );
    }
    case("f<boolean>:(runtime<uint8>){<T>:{a:false&&(runtime>0);b:true||(runtime+1==0);->a<>};v<T>:false;->v};d:@\"debug\";d.print(f(255))", &[]).runs(b"false\n");
}
