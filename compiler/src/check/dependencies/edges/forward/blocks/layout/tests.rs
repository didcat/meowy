use super::{super::tests::checked, *};

#[test]
pub(crate) fn result_layout_validation_keeps_primary_fields_and_unknown_categories() {
    for source in [
        "",
        "v:{->a:1;->b:=true}",
        "flag:=false;|flag|->1",
        "xs:[1];->[xs]",
        "f<never>:(){d:@\"debug\";d.panic(\"stop\")}",
    ] {
        let (mut checker, reports) = checked(source);
        for &id in reports.blocks.keys() {
            checker.validate_result_layout(id, Span::default()).unwrap();
        }
    }
}

#[test]
pub(crate) fn result_layout_validation_rejects_missing_duplicate_and_malformed_slots() {
    for fault in 0..9 {
        let (mut checker, reports) = checked("v:{->a:1;->b:true}");
        let id = *reports
            .blocks
            .iter()
            .find(|(_, (_, value))| value.parent.is_some())
            .unwrap()
            .0;
        let body = checker.bodies.get_mut(&id).unwrap();
        let Layout::Slots(slots) = &mut body.layout else {
            panic!()
        };
        match fault {
            0 => slots[0].field = Some("primary".into()),
            1 => slots[0].mutable = true,
            2 => slots[1].field = None,
            3 => slots[2].field = slots[1].field.clone(),
            4 => slots[1].field = Some(String::new()),
            5 => slots[1].shape = Shape::Union { members: 1 },
            6 => {
                slots.pop();
            }
            7 => body.layout = Layout::Unknown,
            8 => body.layout = Layout::Stopped,
            _ => unreachable!(),
        }
        for port in [Port::BlockNormal(id), Port::BlockResult(id)] {
            assert!(
                checker
                    .validate_block_effect(&reports, 0, port, Span::default())
                    .is_err(),
                "{fault}"
            );
        }
    }
}
