use super::*;
use crate::check::dependencies::PointKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Construction {
    pub(crate) owner: usize,
    pub(crate) capacity: usize,
    pub(crate) count: usize,
    pub(crate) contextual: bool,
    pub(crate) normal: bool,
    pub(crate) control: bool,
    pub(crate) span: Span,
}

impl Checker {
    pub(crate) fn list_producer(
        &mut self,
        capacity: usize,
        count: usize,
        normal: bool,
        span: Span,
    ) -> Result<()> {
        let invalid = || Diagnostic::unsupported("proof list-producer identity mismatch", span);
        if !self.flow.spend(
            self.lists.len().checked_ilog2().unwrap_or(0) as usize * 2
                + self.sequences.len().checked_ilog2().unwrap_or(0) as usize
                + self.list_inputs.len().checked_ilog2().unwrap_or(0) as usize
                + 8,
        ) {
            return Err(Diagnostic::unsupported(
                "proof list-producer budget exhausted",
                span,
            ));
        }
        let id = self.point.ok_or_else(invalid)?;
        let point = self.points.get(id).ok_or_else(invalid)?;
        let sequence = self
            .sequences
            .get(&SequenceSource::Expr(id))
            .ok_or_else(invalid)?;
        let inputs = self.list_inputs.get(&id);
        if point.owner != self.owner
            || point.kind != PointKind::Expr
            || point.span != span
            || sequence.owner != self.owner
            || sequence.items.len() != count
            || inputs.is_some_and(|inputs| inputs.len() != count)
            || count > capacity
            || capacity > crate::list::MAX_CAPACITY
        {
            return Err(invalid());
        }
        let op = Construction {
            owner: self.owner,
            capacity,
            count,
            contextual: inputs.is_some(),
            normal,
            control: self.control,
            span,
        };
        if let Some(prior) = self.lists.get(&id) {
            return if *prior == op { Ok(()) } else { Err(invalid()) };
        }
        self.lists.insert(id, op);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::check;
    use super::*;

    #[test]
    pub(crate) fn list_producers_retain_capacity_source_order_and_distinct_roots() {
        let source =
            "n<uint8>:2;xs:[1,n,3];empty<int32[4]>:[];nested<int32[2][3]>:[[1],[2]];sum:1+2";
        crate::compile(source).unwrap();
        let checker = check(source);
        assert_eq!(checker.lists.len(), 5);
        for (&id, op) in &checker.lists {
            assert!(op.normal && !op.contextual && !op.control);
            assert_eq!(op.owner, 0);
            assert_eq!(op.span, checker.points[id].span);
            assert_eq!(
                op.count,
                checker.sequences[&SequenceSource::Expr(id)].items.len()
            );
        }
        let (&id, op) = checker.lists.first_key_value().unwrap();
        assert_eq!((op.capacity, op.count), (3, 3));
        let items = &checker.sequences[&SequenceSource::Expr(id)].items;
        assert!(items[0] > items[1]);
        for (point, text) in items.iter().zip(["1", "n", "3"]) {
            let span = checker.points[point.unwrap()].span;
            assert_eq!(&source[span.start..span.end], text);
        }
        assert!(
            checker
                .lists
                .values()
                .any(|op| (op.capacity, op.count) == (4, 0))
        );
        assert!(
            checker
                .binaries
                .keys()
                .all(|id| !checker.lists.contains_key(id))
        );
    }

    #[test]
    pub(crate) fn list_producers_keep_contextual_stops_and_outer_coercions_separate() {
        for source in [
            "f:(v<int32><string>){|v<int32>|xs<int32[1]><string[1]>:[v]}",
            "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};xs<int32[2]>:[stop(),1]",
            "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};xs<int32[2]><uint8[2]>:[stop(),{x:300;->x}]",
            "<R>:<{-><never>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;f:(v<R><S>){|v<R>|xs<int32[2]><string[2]>:[v,{x:1;->x}]}",
        ] {
            crate::compile(source).unwrap();
            let checker = check(source);
            let (&id, op) = checker.lists.first_key_value().unwrap();
            assert_eq!(op.contextual, checker.list_inputs.contains_key(&id));
            assert_eq!(op.owner, checker.points[id].owner);
            assert!(!checker.coercions.contains_key(&id));
            if source.contains("stop(),1") {
                assert!(!op.normal);
            }
            if source.contains("<never>;tag") {
                assert!(op.normal);
                assert_eq!(
                    checker.list_inputs[&id][0].kind,
                    crate::check::dependencies::CoercionKind::Stopped
                );
            }
        }
        for (source, code) in [
            ("xs:[]", "E207"),
            ("xs<int32[1]>:[1,2]", "E103"),
            ("xs<uint8[1]>:[256]", "E216"),
        ] {
            let mut checker = Checker::new();
            assert_eq!(
                checker
                    .block(&crate::parser::parse(source).unwrap(), None, None)
                    .unwrap_err()
                    .code,
                code
            );
            assert!(checker.lists.is_empty());
        }
    }

    #[test]
    pub(crate) fn list_producer_replay_and_failures_preserve_headers() {
        let mut checker = check("xs:[1,2]");
        let (&id, &op) = checker.lists.first_key_value().unwrap();
        checker.point = Some(id);
        checker.list_producer(2, 2, true, op.span).unwrap();
        assert_eq!(checker.lists.len(), 1);
        for (capacity, count, normal) in [(1, 2, true), (2, 1, true), (2, 2, false)] {
            assert!(
                checker
                    .list_producer(capacity, count, normal, op.span)
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
            assert_eq!(checker.lists[&id], op);
        }
        checker.lists.clear();
        checker.flow.work = crate::flow::MAX_PROOF_WORK;
        assert!(
            checker
                .list_producer(2, 2, true, op.span)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert!(checker.lists.is_empty());
    }
}
