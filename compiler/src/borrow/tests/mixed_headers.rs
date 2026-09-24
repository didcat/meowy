use super::{accepts, rejects};

#[test]
pub(crate) fn mixed_headers_keep_known_shared_roots_beside_local_exclusive_loans() {
    accepts(
        "<R>:<{n<int32>:=}>;x:1;y:2;p:=&x;old:p;first:=true;i:=0;r<R>:'out{'loop{|first|{'out->n:=7;w:&!n;*w=8;first=false};v:*p;b:*old;p=&y;i=i+1;|i<2|'loop.restart()}};v:r.n",
    );
}

#[test]
pub(crate) fn mixed_headers_allow_inactive_nullable_predecessors() {
    accepts(
        "<R>:<{n<int32>:=}>;<H>:<{view<&int32><null>}>;x:7;empty<H>:{};full<H>:{->view:&x};h:=empty;first:=true;i:=0;r<R>:'out{'loop{|first|{'out->n:=7;w:&!n;*w=8;first=false};|h.view<&int32>|v:*(h.view~<&int32>);h=full;i=i+1;|i<2|'loop.restart()}}",
    );
}

#[test]
pub(crate) fn mixed_headers_cover_nested_restart_targets() {
    accepts(
        "<R>:<{n<int32>:=}>;x:1;y:2;p:=&x;first:=true;i:=0;r<R>:'out{'outer{|first|{'out->n:=7;w:&!n;*w=8;first=false};j:=0;'inner{v:*p;p=&y;j=j+1;|j<2|'inner.restart()};i=i+1;|i<2|'outer.restart()}}",
    );
}

#[test]
pub(crate) fn mixed_headers_retain_exclusive_ancestry_and_physical_conflicts() {
    rejects(
        "<R>:<{n<int32>:=}>;x:1;p:=&x;first:=true;r<R>:'out{'loop{|first|{'out->n:=7;w:&!n;p=&*w;first=false;'loop.restart()};v:*p}}",
        "B001",
    );
    rejects(
        "<R>:<{n<int32>:=}>;x:1;p:=&x;first:=true;r<R>:'out{'loop{|first|{'out->n:=7;p=&n;w:&!n;v:*p;*w=8;first=false;'loop.restart()};v:*p}}",
        "E302",
    );
    accepts(
        "<R>:<{n<int32>:=}>;x:1;p:=&x;first:=true;r<R>:'out{'loop{|first|{'out->n:=7;w:&!n;p=&*w;v:*p;p=&x;first=false;'loop.restart()};v:*p}}",
    );
}

#[test]
pub(crate) fn mixed_headers_do_not_waive_call_or_input_opacity() {
    rejects(
        "<R>:<{n<int32>:=}>;keep<&int32>:(p<&int32>,extra<string>){->p};x:1;y:2;p:=keep(&x,\"\");first:=true;i:=0;r<R>:'out{'loop{|first|{'out->n:=7;w:&!n;*w=8;first=false};v:*p;p=&y;i=i+1;|i<2|'loop.restart()}}",
        "B001",
    );
    rejects(
        "<R>:<{n<int32>:=}>;run<null>:(p<&int32>){s:=p;first:=true;i:=0;r<R>:'out{'loop{|first|{'out->n:=7;w:&!n;*w=8;first=false};v:*s;s=p;i=i+1;|i<2|'loop.restart()}}}",
        "B001",
    );
}

#[test]
pub(crate) fn mixed_headers_preserve_owner_resets_and_captured_store_leave() {
    accepts(
        "<R>:<{n<int32>:=}>;x:1;y:2;p:=&x;i:=0;r<R>:'out{first:=true;'loop{|first|{'out->n:=i;w:&!n;*w=*w+10;p=&y;first=false;'loop.restart()};v:*p};i=i+1;|i<2|'out.restart()}",
    );
    accepts(
        "<R>:<{n<int32>:=}>;run<null>:(stop<boolean>){x:1;y:2;p:=&x;first:=true;i:=0;r<R>:'out{'loop{|first|{'out->n:=3;w:&!n;*w={|stop|'out.leave();->9};first=false};v:*p;p=&y;i=i+1;|i<2|'loop.restart()}}}",
    );
}
