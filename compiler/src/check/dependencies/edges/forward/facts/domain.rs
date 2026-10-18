use super::*;

mod integers;

pub(super) fn budget(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-domain budget exhausted", span)
}

pub(super) fn invalid(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-domain identity mismatch", span)
}
