mod command;
mod markup;
pub(crate) mod model;
mod render;

use crate::{diagnostic::Diagnostic, hir, parser};
pub(crate) use command::execute;
pub(crate) use markup::{Example, ExampleMode, Link, value_path};
pub(crate) use model::{Entry, Kind, Model};

pub(crate) type Result<T> = std::result::Result<T, Diagnostic>;
pub(crate) const MAX_SOURCE: usize = 4 * 1024 * 1024;
pub(crate) const MAX_WORK: usize = 2 * 1024 * 1024;

pub(crate) fn checked(
    source: &str,
    collect: bool,
) -> std::result::Result<(hir::Program, Option<Model>), Vec<Diagnostic>> {
    let parsed = parser::parse_documented(source)?;
    let docs = if collect || !parsed.docs.is_empty() {
        Some(Model::new(source, &parsed).map_err(|error| vec![error])?)
    } else {
        None
    };
    crate::check::check_documented(&parsed.block, docs)
}

#[cfg(test)]
mod tests;
