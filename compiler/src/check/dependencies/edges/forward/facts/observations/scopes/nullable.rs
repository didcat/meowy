use super::super::tests::context;
use super::*;
use crate::{check::dependencies::bodies::Layout, hir::Type};

#[test]
pub(crate) fn nullable_scope_headers_preserve_boolean_and_exact_integer_domains() {
    let mut types = vec![("boolean".to_owned(), Type::Bool)];
    for bits in [8, 16, 32, 64] {
        for signed in [false, true] {
            let name = format!("{}int{bits}", if signed { "" } else { "u" });
            types.push((name, Type::Int { bits, signed }));
        }
    }
    for (name, ty) in types {
        let value = if ty == Type::Bool { "true" } else { "7" };
        let source = format!("f<{name}><null>:(flag<boolean>){{|flag|->{value}}}");
        let (mut checker, reports, ports) = context(&source);
        let block = reports.entries[&1].0;
        assert_eq!(checker.bodies[&block].layout, Layout::Unknown);
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let definitions = definitions::Definitions::default();
        let (obs, parts) = checker
            .scalar_scope_observations(
                &reports,
                &graph,
                Observations::new(&definitions),
                Span::default(),
                MAX_EDGES,
            )
            .unwrap();
        assert!(obs.scopes[&block].scalar.nullable);
        let (obs, _) = checker
            .scalar_slot_observations(&reports, &graph, obs, Span::default(), parts)
            .unwrap();
        let expected = domain::Domain::declared(
            &Type::Union(vec![ty, Type::Null]),
            &mut checker.flow,
            Span::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(obs.slots[&block].declared, expected);
        assert_eq!(obs.slots[&block].owner, 1);
        assert_eq!(obs.slots[&block].control, Some(false));
    }
}

#[test]
pub(crate) fn nullable_scope_headers_do_not_infer_domains_from_other_two_member_unions() {
    let source = "f:(flag<boolean>,text<string>,n<float64>,view<&int32>){a:{|flag|->text};b:{|flag|->n};c:{|flag|->view};d<boolean><int32>:{->true}}";
    let (mut checker, reports, ports) = context(source);
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let (obs, parts) = checker
        .scalar_scope_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let (obs, _) = checker
        .scalar_slot_observations(&reports, &graph, obs, Span::default(), parts)
        .unwrap();
    let blocks = checker
        .bodies
        .iter()
        .filter(|(_, body)| body.parent.is_some())
        .collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    for (&block, body) in blocks {
        assert_eq!(body.scalar, None);
        assert!(!obs.scopes.contains_key(&block));
        assert!(!obs.slots.contains_key(&block));
        assert!(
            !obs.forwarded
                .contains_key(&Port::Normal(body.parent.unwrap()))
        );
    }
}

#[test]
pub(crate) fn nullable_scope_headers_reject_stale_descriptors_and_layouts() {
    for fault in 0..4 {
        let (mut checker, mut reports, ports) = context("f:(flag<boolean>){v:{|flag|->7}}");
        let block = *checker
            .bodies
            .iter()
            .find(|(_, body)| body.scalar.is_some_and(|scalar| scalar.nullable))
            .unwrap()
            .0;
        match fault {
            0 => {
                reports
                    .blocks
                    .get_mut(&block)
                    .unwrap()
                    .1
                    .scalar
                    .as_mut()
                    .unwrap()
                    .kind = crate::check::dependencies::ScalarKind::Bool
            }
            1 => {
                reports
                    .blocks
                    .get_mut(&block)
                    .unwrap()
                    .1
                    .scalar
                    .as_mut()
                    .unwrap()
                    .nullable = false
            }
            2 => checker.bodies.get_mut(&block).unwrap().scalar = None,
            3 => checker.bodies.get_mut(&block).unwrap().layout = Layout::Stopped,
            _ => unreachable!(),
        }
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let definitions = definitions::Definitions::default();
        let saved = format!("{reports:?}{graph:?}{:?}", checker.bodies);
        assert!(
            checker
                .scalar_scope_observations(
                    &reports,
                    &graph,
                    Observations::new(&definitions),
                    Span::default(),
                    MAX_EDGES,
                )
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}{graph:?}{:?}", checker.bodies), saved);
    }
}

#[test]
pub(crate) fn nullable_scope_headers_reserve_exact_work_and_payload() {
    let (mut checker, reports, ports) =
        context("f:(flag<boolean>){a:{|flag|->7};b:{|flag|->true}}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let start = checker.flow.work;
    let (expected, left) = checker
        .scalar_scope_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let cost = checker.flow.work - start;
    let used = MAX_EDGES - left;
    let saved = format!("{reports:?}{graph:?}");
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = checker.scalar_scope_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            room,
        );
        assert_eq!(result.is_ok(), pass);
        if let Ok((obs, left)) = result {
            assert_eq!(left, 0);
            assert_eq!(obs.scopes, expected.scopes);
            assert_eq!(obs.forwarded, expected.forwarded);
        }
        assert_eq!(format!("{reports:?}{graph:?}"), saved);
    }
}
