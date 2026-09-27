use super::{
    PointKind,
    edges::{Edge, Port, Route},
};
use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LocalRead {
    pub(crate) owner: usize,
    pub(crate) local: hir::LocalId,
    pub(crate) storage: hir::LocalId,
    pub(crate) normal: bool,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn capture_local_read(
        &mut self,
        id: hir::PointId,
        local: hir::LocalId,
        normal: bool,
        span: Span,
    ) -> Result<()> {
        let invalid = || Diagnostic::unsupported("proof local-read identity mismatch", span);
        if !self.flow.spend(
            self.local_reads.len().checked_ilog2().unwrap_or(0) as usize * 2
                + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                + 4,
        ) {
            return Err(Diagnostic::unsupported(
                "proof local-read budget exhausted",
                span,
            ));
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        let storage = self
            .proofs
            .aliases
            .get(&local)
            .map_or(local, |alias| alias.root);
        if point.kind != PointKind::Expr
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || self.locals.get(local).is_none()
            || self.locals.get(storage).is_none()
        {
            return Err(invalid());
        }
        let mut edges = vec![Edge::new(Port::Entry(id), Port::Operation(id), Route::Next)];
        if normal {
            edges.push(Edge::new(
                Port::Operation(id),
                Port::Normal(id),
                Route::Next,
            ));
        }
        let op = LocalRead {
            owner: self.owner,
            local,
            storage,
            normal,
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.local_reads.get(&id) {
            return if *prior == op { Ok(()) } else { Err(invalid()) };
        }
        if !self.edge_room(op.edges.len()) {
            return Err(Diagnostic::unsupported(
                "proof local-read budget exhausted",
                span,
            ));
        }
        self.local_read_edges += op.edges.len();
        self.local_reads.insert(id, op);
        Ok(())
    }
}

#[cfg(test)]
mod stages;

#[cfg(test)]
mod modules;

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn check(source: &str) -> Checker {
        let mut checker = Checker::new();
        checker
            .block(&crate::parser::parse(source).unwrap(), None, None)
            .unwrap();
        checker
    }

    #[test]
    pub(crate) fn local_read_capture_retains_plain_mutable_parameter_and_raw_root_identity() {
        let source = "a:1;b:=2;c:a;d:b;f<int32>:(p<int32>){->p}";
        crate::compile(source).unwrap();
        let checker = check(source);
        assert_eq!(checker.local_reads.len(), 3);
        for ((&id, op), name) in checker.local_reads.iter().zip(["a", "b", "p"]) {
            assert_eq!(&source[op.span.start..op.span.end], name);
            assert_eq!(op.local, op.storage);
            assert!(op.normal);
            assert_eq!(op.owner, checker.points[id].owner);
            let outer = checker.points[id].parent.unwrap();
            assert_eq!(checker.narrowings[&outer].input, id);
        }
        assert!(checker.local_reads.values().any(|op| op.owner != 0));
    }

    #[test]
    pub(crate) fn local_read_capture_canonicalizes_slot_aliases_without_pointee_reads() {
        let source = "c:=false;row:'out{|c|{'out->value:=1;x:value};|!c|{'out->value:=1;y:value}}";
        crate::compile(source).unwrap();
        let checker = check(source);
        let aliases: Vec<_> = checker
            .local_reads
            .values()
            .filter(|op| checker.proofs.aliases.contains_key(&op.local))
            .collect();
        assert_eq!(aliases.len(), 2);
        assert_ne!(aliases[0].local, aliases[1].local);
        assert_eq!(aliases[0].storage, aliases[1].storage);
        let source = "n:=1;p:&n;q:p;x:*q";
        crate::compile(source).unwrap();
        let checker = check(source);
        assert_eq!(checker.local_reads.len(), 2);
        for op in checker.local_reads.values() {
            assert!(matches!(checker.locals[op.local], hir::Type::Reference(_)));
            assert_eq!(op.local, op.storage);
            assert_ne!(op.storage, 0);
        }
    }

    #[test]
    pub(crate) fn local_read_capture_preserves_never_and_required_static_boundaries() {
        let source = "f<never>:(p<never>){->p}";
        crate::compile(source).unwrap();
        let checker = check(source);
        assert_eq!(checker.local_reads.len(), 1);
        assert!(!checker.local_reads.values().next().unwrap().normal);
        let checker = check("p:@\"proof\";n:2;xs<int32[n]>:[1];revision:p.revision");
        assert!(checker.local_reads.is_empty());
        for (source, code) in [("x:x", "E201"), ("x:1;f:(){x}", "B001")] {
            let mut checker = Checker::new();
            assert_eq!(
                checker
                    .block(&crate::parser::parse(source).unwrap(), None, None)
                    .unwrap_err()
                    .code,
                code
            );
            assert!(checker.local_reads.is_empty());
        }
    }
}
