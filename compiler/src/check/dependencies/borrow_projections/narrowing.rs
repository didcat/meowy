use super::{tests::check, *};

#[test]
pub(crate) fn projection_narrowing_captures_guarded_reference_and_nested_owned_fields() {
    for (source, flags) in [
        (
            "<R>:<{n<int32>}>;r<R>:{->n:1};h:{->p<&R><null>:&r};|h.p<&R>|q:&(h.p.n)",
            vec![true],
        ),
        (
            "<R>:<{n<int32>}>;<H>:<{p<&R>}>;r<R>:{->n:1};h:{->inner<H><null>:{->p:&r}};|h.inner<H>|q:&(h.inner.p.n)",
            vec![true, false],
        ),
        (
            "<R>:<{n<int32>}>;<H>:<{p<&R><null>}>;r<R>:{->n:1};h:{->inner<H><null>:{->p:&r}};|h.inner<H>|{|h.inner.p<&R>|q:&(h.inner.p.n)}",
            vec![true, true],
        ),
    ] {
        crate::compile(source).unwrap();
        let checker = check(source);
        assert_eq!(checker.projections.len(), 1);
        let (&id, plan) = checker.projections.first_key_value().unwrap();
        let fields: Vec<_> = plan
            .steps
            .iter()
            .filter_map(|step| match step {
                Step::Field { narrow, .. } => Some(*narrow),
                _ => None,
            })
            .collect();
        assert_eq!(fields, flags);
        assert_eq!(plan.steps.len(), flags.len() + 1);
        assert_eq!(plan.steps.last(), Some(&Step::Address(0)));
        assert_eq!(plan.mode, Some(hir::ReferenceMode::Shared));
        assert_eq!(checker.points[plan.parent].parent, Some(id));
        assert!(plan.site.is_some());
    }
}

#[test]
pub(crate) fn projection_narrowing_preserves_plain_fields_and_loaded_paths() {
    for source in [
        "r:{->n:1};h:{->p:&r};q:&(h.p.n)",
        "r:{->n:1};h:{->p:&r};v:&h;q:&(v.p.n)",
    ] {
        crate::compile(source).unwrap();
        let checker = check(source);
        assert!(checker.projections.values().all(|plan| {
            plan.steps
                .iter()
                .all(|step| !matches!(step, Step::Field { narrow: true, .. }))
        }));
    }
}

#[test]
pub(crate) fn projection_narrowing_preserves_stops_and_existing_rejections() {
    let source = "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};q:&(stop().missing)";
    crate::compile(source).unwrap();
    let checker = check(source);
    let plan = checker.projections.values().next().unwrap();
    assert!(plan.steps.is_empty() && plan.site.is_none());
    for (source, code) in [
        (
            "<R>:<{n<int32>}>;r<R>:{->n:1};h:{->p<&R><null>:&r};q:&(h.p.n)",
            "B001",
        ),
        (
            "<R>:<{n<int32>}>;r<R>:{->n:1};h:{->p<&R><null>:&r};|h.p<&R>|q:&(h.p.missing)",
            "E201",
        ),
        ("r:={->n:1};h:{->p:&r};q:&(h.p.n);r={->n:2};x:*q", "E302"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}
