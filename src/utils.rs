use std::{fmt::Display, path::PathBuf};

use anyhow::{Context, Result};
use handlebars::Handlebars;
use serde::Serialize;

pub(crate) struct Ctx<'l> {
    pub(crate) src: PathBuf,
    pub(crate) reg: Handlebars<'l>,
    pub(crate) dest: PathBuf,
    pub(crate) any_error: bool,
}

impl<'l> Ctx<'l> {
    pub(crate) fn print_error(&mut self, err: impl Display) {
        self.any_error = true;
        eprintln!("{err:#}");
    }
}

pub(crate) fn render<T>(reg: &Handlebars, name: &str, data: &T) -> Result<String>
where
    T: Serialize,
{
    reg.render(name, data)
        .with_context(|| format!("failed to render template {name}"))
}
