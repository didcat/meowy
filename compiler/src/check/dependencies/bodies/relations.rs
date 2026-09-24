use super::{Body, Fact, Link, Role, tests::check};
use crate::hir;

pub(crate) fn at(body: &Body, fact: Fact) -> usize {
    body.facts
        .iter()
        .position(|(item, _)| *item == fact)
        .unwrap()
}

pub(crate) fn link(body: &Body, child: usize, parent: usize, role: Role) {
    assert_eq!(body.links[child], Some(Link { parent, role }));
}

#[test]
pub(crate) fn body_relations_connect_call_operands_and_branch_writes() {
    let source = "f<int32>:(v<int32>){->v};flag:=false;x:=1;'loop{a:f(x+1);|flag|x=a;|!flag|'loop.restart();b:x}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let body = &checker.bodies[&checker.restart_inputs[&0].target];
    let call = at(body, Fact::Call(0));
    link(body, call, at(body, Fact::Bind(3)), Role::Data);
    link(body, at(body, Fact::Read(2)), call, Role::Data);
    let branch = at(body, Fact::Branch);
    link(body, at(body, Fact::Read(1)), branch, Role::Condition);
    let write = at(body, Fact::Write(2));
    link(body, write, branch, Role::Then);
    link(body, at(body, Fact::Read(3)), write, Role::Data);
    assert_eq!(body.links[at(body, Fact::Bind(4))], None);
    assert!(checker.derived.is_empty());
}

#[test]
pub(crate) fn body_relations_keep_index_and_indirect_address_inputs_distinct() {
    let source = "i:1;x:=3;xs<int32[2]>:=[1,2];xs[i]=x;r:&!x;*r=xs[i]";
    crate::compile(source).unwrap();
    let (checker, root) = check(source);
    let body = &checker.bodies[&root.id];
    let write = at(body, Fact::Write(2));
    link(body, at(body, Fact::Read(0)), write, Role::Index);
    link(body, at(body, Fact::Read(1)), write, Role::Data);
    let store = at(body, Fact::Store);
    link(body, at(body, Fact::Read(3)), store, Role::Address);
    link(body, at(body, Fact::Read(2)), store, Role::Data);
    assert_eq!(body.links[store], None);
}

#[test]
pub(crate) fn body_relations_preserve_both_arms_and_nested_body_boundaries() {
    let source = "flag:=false;x:=1;|flag|x=2;|flag|{copy:x}";
    let (mut checker, mut root) = check(source);
    let hir::Stmt::If {
        then, otherwise, ..
    } = &mut root.stmts[2]
    else {
        panic!()
    };
    *otherwise = then.clone();
    root.id = checker.block;
    checker
        .track_body(&root, crate::ast::Span::default())
        .unwrap();
    let body = &checker.bodies[&root.id];
    let branch = at(body, Fact::Branch);
    let writes = body
        .facts
        .iter()
        .enumerate()
        .filter(|(_, (fact, _))| *fact == Fact::Write(1))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    assert_eq!(writes.len(), 2);
    link(body, writes[0], branch, Role::Then);
    link(body, writes[1], branch, Role::Else);
    let (index, child) = body
        .facts
        .iter()
        .enumerate()
        .find_map(|(index, (fact, _))| match fact {
            Fact::Block(id) => Some((index, *id)),
            _ => None,
        })
        .unwrap();
    assert_eq!(body.links[index].unwrap().role, Role::Then);
    assert_eq!(checker.bodies[&child].links[0], None);
}

#[test]
pub(crate) fn body_relations_are_bounded_backward_links_with_stable_identity() {
    let source = "x:1;y:x+{->x};f<int32>:(v<int32>){->v};z:f(y)";
    let (mut checker, root) = check(source);
    for body in checker.bodies.values() {
        assert_eq!(body.links.len(), body.facts.len());
        for (index, link) in body.links.iter().enumerate() {
            assert!(link.is_none_or(|link| link.parent < index));
        }
    }
    let count = checker.body_facts;
    checker
        .track_body(&root, crate::ast::Span::new(0, source.len()))
        .unwrap();
    assert_eq!(checker.body_facts, count);
    assert_eq!(
        checker.bodies[&root.id].links.len(),
        checker.bodies[&root.id].facts.len()
    );
}
