use super::Case;

#[test]
pub fn emitted_borrows_address_live_slots_and_release_at_last_use() {
    Case::new(
        r#"
d:@"debug"
value:{
    ->n:=1
    copy:n
    view:&n
    same:&n
    d.print(view==same);d.print(view==&copy);d.print(*view)
    n=2
    later:&n
    d.print(*later)
    n=*later+1
}
d.print(value.n)
"#,
    )
    .runs(b"true\nfalse\n1\n2\n3\n");
}

#[test]
pub fn emitted_borrows_follow_target_lifetime_beyond_alias_scope() {
    Case::new(
        r#"
d:@"debug"
value:'result{
    view:{'result->n:=7;->&n}
    d.print(*view)
    ->seen:*view
}
d.print(value.n);d.print(value.seen)
other:'result{
    view:{'result->child:={->n:=8};->&(child.n)}
    d.print(*view)
}
d.print(other.child.n)
"#,
    )
    .runs(b"7\n7\n7\n8\n8\n");
}

#[test]
pub fn emitted_borrows_preserve_field_and_collection_conflict_regions() {
    Case::new(
        r#"
d:@"debug"
first<&int32>:(items<&int32[3]>){->&(items[1])}
value:{
    ->child:={->left:=1;->right:=2}
    view:&(child.left)
    child.right=3
    d.print(*view);d.print(child.right)
    child.left=4
    ->items<int32[3]>:=[5,6]
    parent:&items
    item:first(parent)
    same:&(items[1])
    d.print(item==same);d.print(*item)
    items[1]=7
    items=items.add(8)
    ->other:=9
    held:&other
    items[2]=10
    d.print(*held)
}
d.print(value.child.left);d.print(value.items[1]);d.print(value.items[2]);d.print(value.items[3])
"#,
    )
    .runs(b"1\n3\ntrue\n5\n9\n4\n7\n10\n8\n");
}

#[test]
pub fn emitted_borrows_address_compatible_widened_payloads() {
    Case::new(
        r#"
d:@"debug"
choose:(flag<boolean>)'result{
    |flag|{'result->n:=1;view:&n;d.print(*view);n=2}
    |!flag|{'result->n:="before";view:&n;d.print(*view);n="after"}
}
one:choose(true).n
two:choose(false).n
|one<int32>|d.print(one~<int32>)
|two<string>|d.print(two~<string>)
maybe:(flag<boolean>)'result{
    |flag|{'result->child:={->n:=3};view:&(child.n);d.print(*view);child.n=4}
}
present:maybe(true).child
absent:maybe(false).child
|present<{n<int32>:=}>|d.print(present.n)
|absent<null>|d.print("absent")
value:{->n<int32><null>:=null;view:&n;copy:*view;|copy<null>|d.print("null");n=5}
|value.n<int32>|d.print(value.n~<int32>)
"#,
    )
    .runs(b"1\nbefore\n2\nafter\n3\n4\nabsent\nnull\n5\n");
}

#[test]
pub fn emitted_borrows_preserve_restart_and_discarded_storage_lifetimes() {
    Case::new(
        r#"
d:@"debug"
count:=0
value:'again{
    ->n:=0
    view:&n
    d.print(*view)
    count=count+1
    n=count
    |count<2|'again.restart()
}
d.print(value.n)
outer:{->n:=3;view:&n;i:=0;'loop{d.print(*view);i=i+1;|i<2|'loop.restart()};n=4}
d.print(outer.n)
run<null>:(flag<boolean>){
    'escape{
        value:'target{
            |flag|{'target->n:=5;view:&n;d.print(*view);n=6;d.print(n);'escape.leave()}
            ->"done"
        }
        d.print(value)
    }
}
run(true);run(false)
nested<null>:(flag<boolean>){
    'escape{
        value:'target{
            |flag|{view:{'target->lost:=7;->&lost};d.print(*view);'escape.leave()}
            ->"kept"
        }
        d.print(value)
    }
}
nested(true);nested(false)
"#,
    )
    .runs(b"0\n0\n2\n3\n3\n4\n5\n6\ndone\n7\nkept\n");
}

#[test]
pub fn emitted_borrows_reject_live_writes_and_result_publication_escapes() {
    for (source, code) in [
        ("value:{->n:=1;view:&n;n=2;copy:*view}", "E302"),
        (
            "value:{->child:={->n:=1};view:&(child.n);child.n=2;copy:*view}",
            "E302",
        ),
        (
            "value:{->child:={->n:=1};view:&(child.n);child={->n:=2};copy:*view}",
            "E302",
        ),
        (
            "value:{->items<int32[2]>:=[1,2];view:&(items[1]);items[2]=3;copy:*view}",
            "E302",
        ),
        (
            "value:{->items<int32[2]>:=[1];view:&(items[{items=[];->1}])}",
            "E302",
        ),
        ("value:{->n:=1;view:{->&n};n=2;copy:*view}", "E302"),
        ("value:{->n:=1;same:&n=={n=2;->&n}}", "E302"),
        (
            "value:{->items:=[{->a:=1;->b:=2}];view:&(items[1].a);items[1].b=3;copy:*view}",
            "E302",
        ),
        (
            "value:{->n:=1;view:&n;i:=0;'loop{copy:*view;n=2;i=i+1;|i<2|'loop.restart()}}",
            "E302",
        ),
        ("value:{->n:=1;->view:&n}", "E303"),
        ("view:{->n:=1;->&n}", "E303"),
        ("value:{->child:={->n:=1};->view:&(child.n)}", "E303"),
        ("value:{->items<int32[2]>:=[1];->view:&(items[1])}", "E303"),
        ("value:'out{view:{'out->n:=1;->&n};->saved:view}", "E303"),
        (
            "bad<{n<int32>:=;view<&int32>}>:(){->n:=1;->view:&n}",
            "E303",
        ),
    ] {
        let case = Case::new(source);
        for profile in ["debug", "release"] {
            let output = case.command("build", &["--json", "--profile", profile]);
            assert_eq!(output.status.code(), Some(1), "{source}");
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(
                error.contains(&format!("\"code\":\"{code}\"")),
                "{source}: {error}"
            );
            assert!(!case.path.join("build").exists());
        }
    }
}

#[test]
pub fn emitted_element_borrows_keep_checked_bounds_and_prior_effects() {
    let source = "d:@\"debug\";value:{->items<int32[2]>:=[1];index:=2;view:&(items[{d.print(\"index\");->index}])}";
    let access = "&(items[{d.print(\"index\");->index}])";
    let start = source.find(access).unwrap();
    let end = start + access.len();
    let case = Case::new(source);
    for profile in ["debug", "release"] {
        let output = case.command("run", &["--profile", profile]);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, b"index\n");
        assert_eq!(
            output.stderr,
            format!(
                "panic[P001]: index 2 is outside initialized length 1 at bytes {start}..{end}\n"
            )
            .as_bytes()
        );
    }
}

#[test]
pub fn emitted_borrows_keep_unrepresented_storage_boundaries_explicit() {
    for source in [
        "owner:1;n:=2;value:'out{'loop{'out->view:=&owner;view=&owner;n=n-1;|n>0|'loop.restart()}}",
        "value:{->n<int32><null>:=1;view:&!n}",
        "choose:(flag<boolean>)'out{|flag|{'out->n<int32><null>:=null;view:&n};|!flag|{'out->n:=\"text\"}}",
    ] {
        let case = Case::new(source);
        for profile in ["debug", "release"] {
            let output = case.command("build", &["--json", "--profile", profile]);
            assert_eq!(output.status.code(), Some(1), "{source}");
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains("\"code\":\"B001\""), "{source}: {error}");
            assert!(!case.path.join("build").exists());
        }
    }
}
