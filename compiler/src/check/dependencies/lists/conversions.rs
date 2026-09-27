use super::*;
use crate::check::dependencies::CoercionKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Input {
    pub(crate) point: hir::PointId,
    pub(crate) primary: bool,
    pub(crate) kind: CoercionKind,
}

impl Checker {
    pub(crate) fn contextual_list_sequence(
        &mut self,
        id: hir::PointId,
        inputs: Vec<Input>,
        normal: bool,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof list-conversion budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof list-conversion identity mismatch", span);
        if inputs.len() > super::super::sequences::MAX_ITEMS
            || !self.flow.spend(
                inputs.len() * 2 + self.list_inputs.len().checked_ilog2().unwrap_or(0) as usize + 4,
            )
        {
            return Err(budget());
        }
        if (!normal
            && !inputs
                .iter()
                .any(|input| input.kind == CoercionKind::Stopped))
            || (normal
                && inputs
                    .iter()
                    .any(|input| input.kind == CoercionKind::Stopped && !input.primary))
        {
            return Err(invalid());
        }
        let key = SequenceSource::Expr(id);
        let mut sequence = self.prepare_sequence(
            key,
            inputs.iter().map(|input| Some(input.point)).collect(),
            span,
        )?;
        sequence.edges.clear();
        let mut edges = Vec::new();
        let mut from = Port::Entry(id);
        let mut stopped = false;
        for (part, input) in inputs.iter().enumerate() {
            let entry = Edge::new(from, Port::Entry(input.point), Route::Next);
            if part == 0 {
                edges.push(entry);
            } else {
                sequence.edges.push(entry);
            }
            from = Port::Normal(input.point);
            if input.primary {
                let stage = Port::Projection {
                    point: id,
                    step: part,
                };
                edges.push(Edge::new(from, stage, Route::Next));
                from = stage;
            }
            if input.kind == CoercionKind::Stopped {
                stopped = true;
                break;
            }
            if input.kind == CoercionKind::Convert {
                let stage = Port::Conversion { point: id, part };
                edges.push(Edge::new(from, stage, Route::Next));
                from = stage;
            }
        }
        if normal && !stopped {
            edges.extend([
                Edge::new(from, Port::Operation(id), Route::Next),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
            ]);
        }
        match (
            self.list_inputs.get(&id),
            self.sequences.get(&key),
            self.endpoints.get(&key),
        ) {
            (Some(prior), Some(seq), Some(ends))
                if *prior == inputs && *seq == sequence && *ends == edges =>
            {
                return Ok(());
            }
            (None, None, None) => (),
            _ => return Err(invalid()),
        }
        if !self.edge_room(sequence.edges.len() + edges.len()) {
            return Err(budget());
        }
        self.sequence_edges += sequence.edges.len();
        self.endpoint_edges += edges.len();
        self.sequences.insert(key, sequence);
        self.endpoints.insert(key, edges);
        self.list_inputs.insert(id, inputs);
        Ok(())
    }
}

#[cfg(test)]
mod stages;

#[cfg(test)]
mod tests {
    use super::super::tests::check;
    use super::*;

    #[test]
    pub(crate) fn contextual_list_plans_capture_only_final_primary_and_union_conversions() {
        for (types, param, narrowed, primary) in [
            ("", "int32><string", "int32", false),
            (
                "<R>:<{-><int32>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;",
                "R><S",
                "R",
                true,
            ),
        ] {
            for (target, kind) in [
                ("int32", CoercionKind::Forward),
                ("T", CoercionKind::Convert),
            ] {
                let source = format!(
                    "<T>:<int32><boolean>;{types}f:(v<{param}>){{|v<{narrowed}>|xs<{target}[1]><string[1]>:[v]}}"
                );
                crate::compile(&source).unwrap();
                let checker = check(&source);
                assert_eq!(checker.list_inputs.len(), 1, "{source}");
                let (&id, inputs) = checker.list_inputs.first_key_value().unwrap();
                assert_eq!(inputs.len(), 1);
                assert_eq!((inputs[0].primary, inputs[0].kind), (primary, kind));
                let point = &checker.points[inputs[0].point];
                assert_eq!(point.parent, Some(id));
                assert_eq!(&source[point.span.start..point.span.end], "v");
                assert_eq!(
                    checker.sequences[&SequenceSource::Expr(id)].items,
                    [Some(inputs[0].point)]
                );
            }
        }
    }

    #[test]
    pub(crate) fn contextual_list_plans_preserve_deferred_order_and_existing_conversions() {
        let source = "<T>:<uint8><boolean>;<U>:<uint16><boolean>;f:(v<uint8><uint16>){|v<uint8>|xs<T[3]><U[3]>:[1,v,3]}";
        crate::compile(source).unwrap();
        let checker = check(source);
        let inputs = checker.list_inputs.values().next().unwrap();
        assert_eq!(
            inputs.iter().map(|input| input.kind).collect::<Vec<_>>(),
            [
                CoercionKind::Forward,
                CoercionKind::Convert,
                CoercionKind::Forward
            ]
        );
        assert!(inputs.iter().all(|input| !input.primary));
        for (input, text) in inputs.iter().zip(["1", "v", "3"]) {
            let span = checker.points[input.point].span;
            assert_eq!(&source[span.start..span.end], text);
        }
    }

    #[test]
    pub(crate) fn contextual_list_plans_keep_stopped_sources_and_candidate_errors() {
        for (source, primary) in [
            (
                "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};xs<int32[2]><uint8[2]>:[stop(),{x:300;->x}]",
                false,
            ),
            (
                "<R>:<{-><never>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;f:(v<R><S>){|v<R>|xs<int32[2]><string[2]>:[v,{x:1;->x}]}",
                true,
            ),
        ] {
            crate::compile(source).unwrap();
            let checker = check(source);
            let inputs = checker.list_inputs.values().next().unwrap();
            assert_eq!(
                (inputs[0].kind, inputs[0].primary),
                (CoercionKind::Stopped, primary)
            );
        }
        for (source, code) in [
            ("xs<int32[1]><string[1]>:[1,2]", "E103"),
            ("xs<uint8[1]><uint16[1]>:[1]", "E207"),
        ] {
            let mut checker = Checker::new();
            assert_eq!(
                checker
                    .block(&crate::parser::parse(source).unwrap(), None, None)
                    .unwrap_err()
                    .code,
                code
            );
            assert!(checker.list_inputs.is_empty());
        }
    }
}
