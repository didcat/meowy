use super::super::versions::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_copy_inputs_qualify_grouped_literals_reads_and_parameter_versions() {
    let (mut checker, program, reports) =
        context("a:(7);b:((a));c:b~<int32>;f<int32>:(n<int32>){copy:n;->copy}");
    let (_, declared, parts) = checker
        .scalar_seed_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (versions, parts) = checker
        .scalar_version_seeds(&program, &reports, &declared, Span::default(), parts)
        .unwrap();
    let (inputs, _) = checker
        .scalar_copy_inputs(&reports, &versions, Span::default(), parts)
        .unwrap();
    assert_eq!(inputs.len(), 4);
    assert_eq!(
        inputs
            .values()
            .filter(|input| matches!(input.producer, Producer::Literal(_)))
            .count(),
        1
    );
    for input in inputs.values() {
        if let Producer::Read(point, version) = input.producer {
            assert_eq!(
                version,
                versions[&checker.local_reads[&point].local].version
            );
        }
    }
    assert!(inputs.values().any(|input| matches!(
        input.producer,
        Producer::Read(
            _,
            Version {
                definition: versions::Definition::Parameter(0),
                ..
            }
        )
    )));
}

#[test]
pub(crate) fn scalar_copy_inputs_leave_mutations_conversions_expressions_and_emissions_unavailable()
{
    let (mut checker, program, reports) = context(
        "x:=1;copy:x;a:7;sum:a+0;predicate:a<int32>;maybe<int32><null>:a;from_block:{->a};text:\"cat\";f<int32>:(){->7};call:f()",
    );
    let (_, declared, parts) = checker
        .scalar_seed_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (versions, parts) = checker
        .scalar_version_seeds(&program, &reports, &declared, Span::default(), parts)
        .unwrap();
    let (inputs, _) = checker
        .scalar_copy_inputs(&reports, &versions, Span::default(), parts)
        .unwrap();
    assert_eq!(inputs.len(), 1);
    assert!(matches!(
        inputs.first_key_value().unwrap().1.producer,
        Producer::Literal(_)
    ));
}

#[test]
pub(crate) fn scalar_copy_inputs_reject_stale_reads_and_require_result_observations() {
    for fault in 0..3 {
        let (mut checker, program, mut reports) = context("a:7;copy:a");
        let point = *checker.local_reads.first_key_value().unwrap().0;
        let parent = checker.points[point].parent.unwrap();
        match fault {
            0 => {
                reports.effects.get_mut(&point).unwrap().0 += 1;
            }
            1 => {
                let (_, Effect::Narrowing(value)) = reports.effects.get_mut(&parent).unwrap()
                else {
                    panic!()
                };
                value.result = false;
            }
            2 => {
                reports.effects.remove(&point);
            }
            _ => unreachable!(),
        }
        let (_, declared, parts) = checker
            .scalar_seed_context(&program, &reports, Span::default(), MAX_EDGES)
            .unwrap();
        let (versions, parts) = checker
            .scalar_version_seeds(&program, &reports, &declared, Span::default(), parts)
            .unwrap();
        let result = checker.scalar_copy_inputs(&reports, &versions, Span::default(), parts);
        if fault == 0 {
            assert!(result.unwrap_err().message.contains("identity mismatch"));
        } else {
            assert_eq!(result.unwrap().0.len(), 1);
        }
    }
}
