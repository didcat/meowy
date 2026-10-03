use super::{super::super::tests::checked, *};

#[test]
pub(crate) fn output_reports_validate_sparse_stages_without_inference() {
    for method in ["print", "panic"] {
        let (mut checker, mut reports) = checked(
            &format!("d:@\"debug\";r:{{->7;->tag:true}};d.{method}(\"a{{r}}b{{1}}\")"),
            false,
        );
        let id = *checker.outputs.first_key_value().unwrap().0;
        let Effect::Output(full) = reports.effects[&id].1.clone() else {
            panic!()
        };
        for state in 0..4 {
            if state == 3 && method == "print" {
                continue;
            }
            let mut output = full.clone();
            output.prefix = state == 3;
            output.terminal = state == 2;
            output.parts.retain(|&part, _| part == 1 && state < 2);
            for part in output.parts.values_mut() {
                part.projection = state == 0;
                part.output = state == 1;
            }
            if state == 2 {
                reports.index.operations.insert(id, 0);
            } else {
                reports.index.operations.remove(&id);
            }
            let before = format!("{reports:?}");
            checker
                .validate_output_report(&reports, id, 0, &output, Span::default())
                .unwrap();
            assert_eq!(format!("{reports:?}"), before);
        }
    }
}

#[test]
pub(crate) fn output_reports_reject_header_part_and_terminal_corruption_atomically() {
    for fault in 0..14 {
        let (mut checker, mut reports) = checked(
            "d:@\"debug\";r:{->7;->tag:true};d.print(\"a{r}b{1}\")",
            false,
        );
        let id = *checker.outputs.first_key_value().unwrap().0;
        let Effect::Output(mut output) = reports.effects[&id].1.clone() else {
            panic!()
        };
        match fault {
            0 => output.panic = true,
            1 => output.control = !output.control,
            2 => output.total += 1,
            3 => output.stopped = Some(1),
            4 => output.prefix = true,
            5 => {
                reports.index.operations.remove(&id);
            }
            6 => {
                reports.index.operations.insert(id, 1);
            }
            7 => output.parts.get_mut(&1).unwrap().input = None,
            8 => {
                output
                    .parts
                    .get_mut(&1)
                    .unwrap()
                    .input
                    .as_mut()
                    .unwrap()
                    .point = id
            }
            9 => {
                output
                    .parts
                    .get_mut(&1)
                    .unwrap()
                    .input
                    .as_mut()
                    .unwrap()
                    .primary = false
            }
            10 => output.parts.get_mut(&0).unwrap().projection = true,
            11 => {
                let part = output.parts.get_mut(&1).unwrap();
                part.output = false;
                part.projection = false;
            }
            12 => {
                let part = output.parts.remove(&1).unwrap();
                output.parts.insert(output.total, part);
            }
            13 => {
                output.parts.clear();
                output.terminal = false;
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.outputs);
        assert!(
            checker
                .validate_output_report(&reports, id, 0, &output, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.outputs), before);
    }
}
