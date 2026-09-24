use super::Case;

#[test]
pub fn emissions_keep_executing_and_dispatch_evaluates_once() {
    Case::new(
        r#"
debug:@"debug"
source<int32>:(){debug.print("source");->21;debug.print("after")}
double<int32>:(x<int32>){->x*2}
debug.print(source().(double))
"#,
    )
    .runs(b"source\nafter\n42\n");
}

#[test]
pub fn never_calls_evaluate_arguments_and_end_the_returning_continuation() {
    let source = r#"
d:@"debug"
mark<int32>:(value<int32>){d.print(value);->value}
stop<never>:(left<int32>,right<int32>){d.print(left+right);d.panic("stop")}
sum<int32>:(left<int32>,right<int32>){d.print("sum");->left+right}
d.print(sum(mark(1),stop(mark(2),mark(3))))
d.print("after")
"#;
    let call = r#"d.panic("stop")"#;
    let start = source.find(call).unwrap();
    let case = Case::new(source);
    for profile in ["debug", "release"] {
        let output = case.command("run", &["--profile", profile]);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, b"1\n2\n3\n5\n");
        assert_eq!(
            output.stderr,
            format!(
                "panic[P006]: stop at bytes {start}..{}\n",
                start + call.len()
            )
            .as_bytes()
        );
    }
}

#[test]
pub fn short_circuit_blocks_and_nonreturning_operands_preserve_effects() {
    Case::new(
        r#"
d:@"debug"
d.print(false&&{d.print("unexpected");->true})
d.print(true||{d.print("unexpected");->false})
f<boolean>:(flag<boolean>){->flag&&d.panic("stop")}
g<boolean>:(flag<boolean>){->flag||d.panic("stop")}
d.print(f(false))
d.print(g(true))
'outer {
    value:{'outer.leave()}&&true
    d.print("unexpected")
}
d.print("done")
"#,
    )
    .runs(b"false\ntrue\nfalse\ntrue\ndone\n");
    for expr in [
        "true&&d.panic(\"stop\")",
        "false||d.panic(\"stop\")",
        "d.panic(\"stop\")&&false",
    ] {
        let case = Case::new(&format!(
            "d:@\"debug\";value:{expr};d.print(\"unexpected\")"
        ));
        for profile in ["debug", "release"] {
            let result = case.command("run", &["--profile", profile]);
            assert_eq!(result.status.code(), Some(1), "{expr}");
            assert!(result.stdout.is_empty(), "{expr}");
            let error = String::from_utf8_lossy(&result.stderr);
            assert!(error.contains("P006"), "{expr}: {error}");
        }
    }
}

#[test]
pub fn skipped_block_operators_accept_nonreturning_operands() {
    Case::new(
        r#"
d:@"debug"
x:1;b:true
d.print(false&&(*{d.print(99);->&x}>0))
d.print(true||(!*{d.print(99);->&b}))
d.print(false&&(-{d.print(99);->1}>0))
d.print(true||((@"bits").not({d.print(99);->1})==0))
"#,
    )
    .runs(b"false\ntrue\nfalse\ntrue\n");
}

#[test]
pub fn nonreturning_operator_operands_preserve_prefix_effects() {
    for (expr, expected) in [
        ("mark(1)+d.panic(\"stop\")", b"1\n".as_slice()),
        ("d.panic(\"stop\")+mark(2)", b"".as_slice()),
        ("mark(1)>d.panic(\"stop\")", b"1\n".as_slice()),
        ("d.panic(\"stop\")==mark(2)", b"".as_slice()),
        ("-d.panic(\"stop\")", b"".as_slice()),
        ("!d.panic(\"stop\")", b"".as_slice()),
        ("(@\"bits\").not(d.panic(\"stop\"))", b"".as_slice()),
    ] {
        let source =
            format!("d:@\"debug\";mark<int32>:(n<int32>){{d.print(n);->n}};v:{expr};d.print(99)");
        let case = Case::new(&source);
        for profile in ["debug", "release"] {
            let output = case.command("run", &["--profile", profile]);
            assert_eq!(
                output.status.code(),
                Some(1),
                "{expr}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stdout, expected, "{expr}");
            assert!(
                String::from_utf8_lossy(&output.stderr).starts_with("panic[P006]: stop"),
                "{expr}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

#[test]
pub fn operator_leave_skips_only_the_nonreturning_continuation() {
    Case::new(
        r#"
d:@"debug"
mark<int32>:(){d.print(1);->1}
'out{v:mark()+{'out.leave()};d.print(99)}
'next{v:-{'next.leave()};d.print(99)}
d.print(2)
"#,
    )
    .runs(b"1\n2\n");
}
