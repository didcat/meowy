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

#[test]
pub(crate) fn body_storage_shares_emitted_slots_but_keeps_value_copies_distinct() {
    let source = "flag:=false;row:'out{|flag|{'out -> value:=1;copy:value};|!flag|{'out -> value:=2;value=3;copy:value}}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let aliases = checker
        .proofs
        .aliases
        .iter()
        .filter(|(_, alias)| alias.field == "value")
        .collect::<Vec<_>>();
    assert_eq!(aliases.len(), 2);
    let root = aliases[0].1.root;
    assert_eq!(aliases[1].1.root, root);
    let mut copies = 0;
    let mut reads = 0;
    let mut writes = 0;
    for body in checker.bodies.values() {
        assert_eq!(body.facts.len(), body.storage.len());
        for (index, (fact, _)) in body.facts.iter().enumerate() {
            match fact {
                Fact::Bind(id) | Fact::Alias(id) | Fact::Read(id) | Fact::Write(id)
                    if aliases.iter().any(|(alias, _)| *alias == id) =>
                {
                    assert_eq!(body.storage[index], Some(root));
                    reads += usize::from(matches!(fact, Fact::Read(_)));
                    writes += usize::from(matches!(fact, Fact::Write(_)));
                }
                Fact::Bind(id) => {
                    assert_eq!(body.storage[index], Some(*id));
                    copies +=
                        usize::from(body.facts.iter().enumerate().any(|(child, (fact, _))| {
                            matches!(fact, Fact::Read(_))
                                && body.links[child].is_some_and(|link| link.parent == index)
                        }));
                }
                _ => {}
            }
        }
    }
    assert_eq!((copies, writes), (2, 1));
    assert!(reads >= 2);
}

#[test]
pub(crate) fn body_storage_keeps_reference_cells_separate_from_pointees() {
    let source = "x:=1;r:&!x;*r=2;y:*r";
    crate::compile(source).unwrap();
    let (checker, root) = check(source);
    let body = &checker.bodies[&root.id];
    assert_eq!(body.storage[at(body, Fact::Read(0))], Some(0));
    assert_eq!(body.storage[at(body, Fact::Read(1))], Some(1));
    assert_eq!(body.storage[at(body, Fact::Store)], None);
    assert_eq!(body.storage[at(body, Fact::Bind(2))], Some(2));
}

#[test]
pub(crate) fn required_body_inputs_keep_storage_and_emitted_input_gates() {
    let source = "n:3;<A>:{-><uint8[n]>}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let inputs = checker.body_inputs.values().flatten().collect::<Vec<_>>();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].id, inputs[0].storage);
    assert_eq!(&source[inputs[0].span.start..inputs[0].span.end], "n");
    let source = "row:{->n:3;<A>:{-><uint8[n]>}}";
    let errors = crate::compile(source).unwrap_err();
    assert_eq!(errors[0].code, "B001");
    assert!(errors[0].message.contains("initializer eligibility"));
}
