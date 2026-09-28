use super::*;
use crate::check::dependencies::edges::tests::check;

#[test]
pub(crate) fn stage_ports_reject_out_of_range_and_absent_output_projection_conversion_stages() {
    let mut checker = check("d:@\"debug\";d.print(\"a{({->7;->tag:true})}b\")");
    let (&id, output) = checker.outputs.first_key_value().unwrap();
    let parts = output.parts.len();
    assert!(
        checker
            .port_owner(
                Port::Output {
                    point: id,
                    part: parts - 1
                },
                Span::default()
            )
            .is_ok()
    );
    for port in [
        Port::Output {
            point: id,
            part: parts,
        },
        Port::Projection { point: id, step: 0 },
        Port::Projection {
            point: id,
            step: usize::MAX,
        },
        Port::Conversion { point: id, part: 1 },
        Port::Prefix(id),
        Port::Snapshot(id),
    ] {
        assert!(
            checker
                .port_owner(port, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "{port:?}"
        );
    }
    assert!(
        checker
            .port_owner(Port::Projection { point: id, step: 1 }, Span::default())
            .is_ok()
    );
    let mut checker =
        check("<T>:<int32><boolean>;f:(v<int32><string>){|v<int32>|xs<T[1]><string[1]>:[v]}");
    let id = *checker.list_inputs.first_key_value().unwrap().0;
    assert!(
        checker
            .port_owner(Port::Conversion { point: id, part: 0 }, Span::default())
            .is_ok()
    );
    assert!(
        checker
            .port_owner(Port::Conversion { point: id, part: 1 }, Span::default())
            .is_err()
    );
}

#[test]
pub(crate) fn stage_ports_validate_address_endpoints_and_index_reservations() {
    let mut checker = check("xs<int32[2]>:=[1];xs[1]=2;p:&!(xs[1]);*p=3");
    let path = *checker.paths.first_key_value().unwrap().0;
    let exclusive = *checker.exclusives.first_key_value().unwrap().0;
    for point in [path, exclusive] {
        assert!(
            checker
                .port_owner(Port::Address { point, step: 1 }, Span::default())
                .is_ok()
        );
        assert!(
            checker
                .port_owner(Port::Address { point, step: 2 }, Span::default())
                .is_err()
        );
        assert!(
            checker
                .port_owner(Port::Reserve { point, step: 0 }, Span::default())
                .is_ok()
        );
        assert!(
            checker
                .port_owner(Port::Reserve { point, step: 1 }, Span::default())
                .is_err()
        );
    }
    let mut checker = check("r:{->n:1};p:&(r.n)");
    let point = *checker.place_borrows.first_key_value().unwrap().0;
    assert!(
        checker
            .port_owner(Port::Address { point, step: 1 }, Span::default())
            .is_ok()
    );
    assert!(
        checker
            .port_owner(Port::Reserve { point, step: 0 }, Span::default())
            .is_err()
    );
}

#[test]
pub(crate) fn stage_ports_keep_unreachable_normal_ports_and_bound_selector_work() {
    let mut checker = check("d:@\"debug\";d.panic(\"stop\")");
    let id = *checker.outputs.first_key_value().unwrap().0;
    assert!(
        checker
            .port_owner(Port::Prefix(id), Span::default())
            .is_ok()
    );
    assert!(
        checker
            .port_owner(Port::Normal(id), Span::default())
            .is_ok()
    );
    let counts = checker.edge_counts();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - 1;
    assert!(
        checker
            .port_owner(Port::Output { point: id, part: 0 }, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(checker.edge_counts(), counts);
}
