use crate::check::Checker;

pub(crate) fn check(source: &str) -> Checker {
    let mut checker = Checker::new();
    checker.derived.insert(0);
    let block = crate::parser::parse(source).unwrap();
    checker.block(&block, None, None).unwrap();
    checker
}

#[test]
pub(crate) fn emitted_initializers_and_control_mark_slot_reads() {
    for body in [
        "->value:=seed;|value|r:p.can_copy<uint32>()",
        "->value:seed;copy:value;|copy|r:p.can_copy<uint32>()",
        "|seed|{->value:=false;|value|r:p.can_copy<uint32>()}",
    ] {
        let source = format!("seed:false;p:@\"proof\";row:{{{body}}}");
        let checker = check(&source);
        assert!(checker.queries[0].control, "{source}");
        for (id, alias) in &checker.proofs.aliases {
            if alias.field == "value" {
                assert!(checker.derived_local(*id));
            }
        }
    }
}

#[test]
pub(crate) fn sibling_slot_aliases_share_initializer_and_write_marks() {
    for first in ["seed", "false"] {
        let source = format!(
            "seed:true;c:=false;p:@\"proof\";row:'out{{|c|{{'out -> value:={first}}};|!c|{{'out -> value:=false;value=seed;|value|r:p.can_copy<uint32>()}};->plain:false}};copy:row.value"
        );
        let checker = check(&source);
        let aliases = checker
            .proofs
            .aliases
            .iter()
            .filter(|(_, alias)| alias.field == "value")
            .collect::<Vec<_>>();
        assert_eq!(aliases.len(), 2);
        assert_eq!(aliases[0].1.root, aliases[1].1.root);
        assert!(aliases.iter().all(|(id, _)| checker.derived_local(**id)));
        assert!(checker.queries[0].control);
        assert!(checker.derived_local(checker.locals.len() - 1));
        assert!(
            checker
                .proofs
                .aliases
                .iter()
                .filter(|(_, alias)| alias.field == "plain")
                .all(|(id, _)| !checker.derived_local(*id))
        );
    }
}

#[test]
pub(crate) fn new_sibling_alias_reads_an_existing_mark_without_a_second_write() {
    let source = "seed:true;c:=false;p:@\"proof\";row:'out{|c|{'out -> value:=seed};|!c|{'out -> value:=false;|value|r:p.can_copy<uint32>()}}";
    let checker = check(source);
    assert!(checker.queries[0].control);
}

#[test]
pub(crate) fn invalid_emitted_alias_writes_keep_ordinary_diagnostics() {
    for (body, code) in [
        ("->value:false;value=seed", "E305"),
        ("->value:=7;value=seed", "E207"),
    ] {
        let source = format!("seed:true;row:{{{body}}}");
        let mut checker = Checker::new();
        checker.derived.insert(0);
        let block = crate::parser::parse(&source).unwrap();
        let error = checker.block(&block, None, None).unwrap_err();
        assert_eq!(error.code, code);
        assert!(
            checker
                .proofs
                .aliases
                .keys()
                .all(|id| !checker.derived_local(*id))
        );
    }
}
