use super::{accepts, rejects};

#[test]
pub(crate) fn union_field_mutability_selects_the_numeric_context() {
    let types = "<A>:<{n<uint8>:=}>;<B>:<{n<uint16>}>;";
    for body in [
        "v<A><B>:{->n:=1}",
        "v<A><B>:{->n:1}",
        "v<A><B>:{->n:=255}",
        "v<A><B>:{->n:65535}",
        "v<A><B>:{->{->n:=1}}",
    ] {
        accepts(&format!("{types}{body}"));
    }
    rejects(&format!("{types}v<A><B>:{{->n:=256}}"), "E216");
    rejects(&format!("{types}v<A><B>:{{->n<uint16>:=1}}"), "E207");
    rejects(
        "<A>:<{n<uint8>:=}>;<B>:<{n<uint16>:=}>;v<A><B>:{->n:=1}",
        "E207",
    );
    rejects(
        "<A>:<{-><uint8>;n<int32>:=}>;<B>:<{-><uint16>;n<int32>}>;v<A><B>:{->1;->n:=1}",
        "E207",
    );
}

#[test]
pub(crate) fn field_mutability_is_part_of_inferred_and_declared_shapes() {
    for source in [
        "<R>:<{n<int32>:=;label<string>}>;r<R>:={->n:=1;->label:\"x\"};r.n=2",
        "a:{->n:=1};b:={->a};b.n=2",
        "<R>:<{n<int32><null>:=}>;r<R>:={};r.n=1",
        "<I>:<{n<int32>}>;<M>:<{n<int32>:=}>;r<I><M>:{->n:=1};|r<M>|{n:r~<M>.n}",
        "<I>:<{n<int32>}>;<M>:<{n<int32>:=}>;r<I[1]><M[1]>:[{->n:=1}]",
        "d:@\"debug\";<I>:<{n<int32>}>;<M>:<{n<int32>:=}>;r<I[1]><M[1]>:[{d.print(0);->n:=1}]",
    ] {
        accepts(source);
    }
    for (source, code) in [
        ("<M>:<{n<int32>:=}>;r<M>:{->n:1}", "E206"),
        ("<I>:<{n<int32>}>;r<I>:{->n:=1}", "E206"),
        (
            "f<null>:(flag<boolean>){r:{|flag|->n:=1;|!flag|->n:2}}",
            "E206",
        ),
        ("<M>:<{n<int32>:=}>;a:{->n:1};r<M>:a", "E207"),
        ("a:{->n:1};b:{->n:=1};same:a==b", "E222"),
        (
            "x<uint8>:1;<A>:<{x<uint16>:=;y<uint8>}>;<B>:<{x<uint16>:=;y<uint16>}>;v<A[1]><B[1]>:[{->x:=300;->y:x}]",
            "B001",
        ),
    ] {
        rejects(source, code);
    }
}

#[test]
pub(crate) fn field_assignment_requires_the_selected_slot_to_be_mutable() {
    accepts("a:1;r:{->view:=&a}");
    accepts("r:{->n:=1};r.n=2");
    accepts("r:={->child:{->n:=1}};r.child.n=2");
    accepts("<Bad>:<{view<&int32>:=}>");
    accepts("<Bad>:<{child<{view<&int32>}>:=}>");

    accepts("r:={->child:={->n:=1};->label:\"x\"};r.child.n=2");
    accepts("r:={->n:=1;->other:=2};r.n={r={->n:=3;->other:=4};->5}");
    for (source, code) in [
        ("r:={->n:1};r.n=2", "E305"),
        ("r:={->child:={->n:1}};r.child.n=2", "E305"),
        ("r:={->n:=1};p:&r;p.n=2", "B001"),
    ] {
        rejects(source, code);
    }
}

#[test]
pub(crate) fn exclusive_field_path_limits_apply_before_root_resolution() {
    let span = crate::ast::Span::default();
    let mut value = crate::ast::Expr {
        kind: crate::ast::ExprKind::Name("absent".into()),
        span,
    };
    for _ in 0..=crate::list::MAX_WRITE_PATH {
        value = crate::ast::Expr {
            kind: crate::ast::ExprKind::Field {
                value: Box::new(value),
                name: "inner".into(),
            },
            span,
        };
    }
    let error = crate::check::Checker::new()
        .exclusive_borrow(&value, span)
        .unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("exclusive borrow path budget"));
}
