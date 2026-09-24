use super::Case;

#[test]
pub fn mixed_headers_keep_shared_versions_and_old_copies() {
    Case::new(
        r#"
d:@"debug"
<R>:<{n<int32>:=}>
x:1;y:2;p:=&x;old:p;first:=true;i:=0
r<R>:'out{
    'loop{
        |first|{'out->n:=7;w:&!n;*w=8;first=false}
        d.print(*old);d.print(*p)
        p=&y;i=i+1
        |i<2|'loop.restart()
    }
}
d.print(r.n)
"#,
    )
    .runs(b"1\n1\n1\n2\n8\n");
}

#[test]
pub fn mixed_headers_keep_inactive_nullable_predecessors() {
    Case::new(
        r#"
d:@"debug"
<R>:<{n<int32>:=}>
<H>:<{view<&int32><null>}>
x:7;empty<H>:{};full<H>:{->view:&x};h:=empty;first:=true;i:=0
r<R>:'out{
    'loop{
        |first|{'out->n:=7;w:&!n;*w=8;first=false}
        |h.view<null>|d.print("empty")
        |h.view<&int32>|d.print(*(h.view~<&int32>))
        h=full;i=i+1
        |i<2|'loop.restart()
    }
}
d.print(r.n)
"#,
    )
    .runs(b"empty\n7\n8\n");
}

#[test]
pub fn mixed_headers_preserve_nested_backedges_and_owner_resets() {
    Case::new(
        r#"
d:@"debug"
<R>:<{n<int32>:=}>
x:1;y:2;p:=&x;i:=0
r<R>:'out{
    first:=true;j:=0
    'loop{
        |first|{'out->n:=i;w:&!n;*w=*w+10;first=false}
        k:=0
        'inner{d.print(*p);p=&y;k=k+1;|k<2|'inner.restart()}
        j=j+1
        |j<2|'loop.restart()
    }
    i=i+1
    |i<2|'out.restart()
}
d.print(r.n)
"#,
    )
    .runs(b"1\n2\n2\n2\n2\n2\n2\n2\n11\n");
}

#[test]
pub fn mixed_headers_preserve_leave_before_a_captured_store() {
    Case::new(
        r#"
d:@"debug"
<R>:<{n<int32>:=}>
run<null>:(stop<boolean>){
    x:1;y:2;p:=&x;first:=true;i:=0
    r<R>:'out{
        'loop{
            |first|{'out->n:=3;w:&!n;*w={d.print("rhs");|stop|'out.leave();->9};first=false}
            d.print(*p);p=&y;i=i+1
            |i<2|'loop.restart()
        }
    }
    d.print(r.n)
}
run(false);run(true)
"#,
    )
    .runs(b"rhs\n1\n2\n9\nrhs\n3\n");
}

#[test]
pub fn mixed_headers_keep_exclusive_ancestry_opacity_and_conflict_rejections() {
    for (source, code) in [
        (
            "<R>:<{n<int32>:=}>;x:1;p:=&x;first:=true;r<R>:'out{'loop{|first|{'out->n:=7;w:&!n;p=&*w;first=false;'loop.restart()};v:*p}}",
            "B001",
        ),
        (
            "<R>:<{n<int32>:=}>;x:1;p:=&x;first:=true;r<R>:'out{'loop{|first|{'out->n:=7;p=&n;w:&!n;v:*p;*w=8;first=false;'loop.restart()};v:*p}}",
            "E302",
        ),
        (
            "<R>:<{n<int32>:=}>;keep<&int32>:(p<&int32>,extra<string>){->p};x:1;y:2;p:=keep(&x,\"\");first:=true;i:=0;r<R>:'out{'loop{|first|{'out->n:=7;w:&!n;*w=8;first=false};v:*p;p=&y;i=i+1;|i<2|'loop.restart()}}",
            "B001",
        ),
        (
            "<R>:<{n<int32>:=}>;run<null>:(p<&int32>){s:=p;first:=true;i:=0;r<R>:'out{'loop{|first|{'out->n:=7;w:&!n;*w=8;first=false};v:*s;s=p;i=i+1;|i<2|'loop.restart()}}}",
            "B001",
        ),
    ] {
        let case = Case::new(source);
        for profile in ["debug", "release"] {
            let output = case.command("run", &["--profile", profile]);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).starts_with(&format!("error[{code}]:")),
                "{source}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
