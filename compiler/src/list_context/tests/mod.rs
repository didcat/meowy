mod candidates;
mod effects;
mod numbers;
mod pure;

use super::{MAX_CONTEXTS, MAX_SCALAR_NODES};

pub(crate) fn rejects(source: &str, code: &str) {
    let errors = crate::compile(source).unwrap_err();
    assert_eq!(
        errors[0].code,
        code,
        "{}-byte source: {errors:?}",
        source.len()
    );
}
