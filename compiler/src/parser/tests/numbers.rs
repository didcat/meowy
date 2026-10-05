use super::*;

#[test]
pub(crate) fn numeric_value_names_preserve_their_spelling_and_token_kind() {
    for name in ["1", "01", "0x1", "0Xf", "0b10", "1_000", "1.0", "1e+2"] {
        let block = parse(&format!("{name}<int32>:=7;{name}=8;-> {name}:9")).unwrap();
        assert!(matches!(&block.stmts[0].kind,
            StmtKind::Bind { name: actual, mutable: true, .. } if actual == name));
        assert!(matches!(&block.stmts[1].kind,
            StmtKind::Assign { target, .. }
                if matches!(&target.kind, ExprKind::Int(actual) | ExprKind::Float(actual) if actual.text == name)));
        assert!(matches!(&block.stmts[2].kind,
            StmtKind::Emit { name: Some(actual), .. } if actual == name));
    }
}

#[test]
pub(crate) fn numeric_parameters_members_and_forward_names_parse() {
    for source in [
        "1<(int32)->int32>;1<int32>:(2<int32>){->2}",
        "f:(1<int32>,1.0<float64>){->1}",
        "row<{1<int32>;1.0<float64>}>:{->1:2;->1.0:3.0};row.1;row.1.0",
        "row.&1;row.&!0x1;row.*1e2",
    ] {
        parse(source).unwrap();
    }
}

#[test]
pub(crate) fn numeric_value_names_do_not_relax_tokens_types_or_labels() {
    for source in ["12cat:1", "1__0:1", "0x_ff:1"] {
        assert_eq!(parse(source).unwrap_err()[0].code, "E001");
    }
    for source in ["<1>:<int32>", "x<1>:2", "'1{}", "-1:2"] {
        assert_eq!(parse(source).unwrap_err()[0].code, "E004");
    }
}
