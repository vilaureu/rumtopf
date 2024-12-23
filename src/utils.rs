use std::{collections::HashSet, fmt::Display, path::PathBuf};

use anyhow::{Context, Result};
use handlebars::Handlebars;
use serde::Serialize;

use crate::Recipe;

pub(crate) struct Ctx<'l> {
    pub(crate) src: PathBuf,
    pub(crate) reg: Handlebars<'l>,
    pub(crate) dest: PathBuf,
    pub(crate) any_error: bool,
    pub(crate) title: Option<String>,
    pub(crate) links: Vec<Link>,
    pub(crate) footer: String,
}

impl Ctx<'_> {
    pub(crate) fn print_error(&mut self, err: impl Display) {
        self.any_error = true;
        eprintln!("{err:#}");
    }
}

#[derive(Clone, Serialize)]
pub(crate) struct Link {
    pub(crate) label: String,
    pub(crate) href: String,
}

pub(crate) struct Rtx<'r> {
    pub(crate) recipes: &'r [Recipe],
    pub(crate) default_lang: &'r str,
    pub(crate) langs: Vec<Option<&'r str>>,
}

impl<'r> Rtx<'r> {
    pub(crate) fn new(recipes: &'r [Recipe], default_lang: &'r str) -> Self {
        let mut langs = recipes
            .iter()
            .map(|r| r.lang.as_deref())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        langs.sort_unstable();
        Self {
            recipes,
            default_lang,
            langs,
        }
    }
}

pub(crate) fn render<T>(reg: &Handlebars, name: &str, data: &T) -> Result<String>
where
    T: Serialize,
{
    reg.render(name, data)
        .with_context(|| format!("failed to render template {name}"))
}
