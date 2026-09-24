#[test]
pub(crate) fn dense_reference_liveness_rejects_with_a_bounded_diagnostic() {
    let mut source = "a:=1;".to_owned();
    for id in 0..512 {
        source.push_str(&format!("r{id}:&a;"));
    }
    for id in 0..512 {
        source.push_str(&format!("v{id}:*r{id};"));
    }
    let errors = crate::compile(&source).unwrap_err();
    assert_eq!(errors[0].code, "B001", "{errors:?}");
    assert!(errors[0].message.contains("loan-analysis budget"));
}

#[test]
pub(crate) fn aliases_of_many_origins_cannot_expand_storage_without_bound() {
    let params = (0..63)
        .map(|id| format!("p{id}<boolean>"))
        .collect::<Vec<_>>()
        .join(",");
    let mut source = format!("f<null>:({params}){{");
    for id in 0..64 {
        source.push_str(&format!("a{id}:{id};"));
    }
    source.push_str("r:'pick {");
    for id in 0..63 {
        source.push_str(&format!("|p{id}|{{'pick->&a{id};'pick.leave()}};"));
    }
    source.push_str("->&a63};");
    for (count, budgets) in [
        (2048, vec!["loan-analysis budget"]),
        (
            4096,
            vec!["borrow-origin fact budget", "control-flow proof budget"],
        ),
    ] {
        let mut source = source.clone();
        for id in 0..count {
            source.push_str(&format!("r{id}:r;"));
        }
        source.push('}');
        let errors = crate::compile(&source).unwrap_err();
        assert_eq!(errors[0].code, "B001", "{errors:?}");
        assert!(
            budgets
                .iter()
                .any(|budget| errors[0].message.contains(budget)),
            "{count}: {errors:?}"
        );
    }
}

#[test]
pub(crate) fn component_aliases_obey_the_reference_value_budget() {
    let mut source = "a:=1;r:{".to_owned();
    for id in 0..64 {
        source.push_str(&format!("->field{id}:&a;"));
    }
    source.push_str("};");
    for id in 0..1024 {
        source.push_str(&format!("copy{id}:r;"));
    }
    let errors = crate::compile(&source).unwrap_err();
    assert_eq!(errors[0].code, "B001", "{errors:?}");
    assert!(
        errors[0].message.contains("loan-analysis budget"),
        "{errors:?}"
    );
}
